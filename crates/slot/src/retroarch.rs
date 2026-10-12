use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use slot_store::{atomic_write, Cart, Core, Layout};

use crate::core::dylib_name;
use crate::link_net::Cancel;
use crate::link_radio::{LinkRole, RadioFail};

pub const REQUEST_FILE: &str = "Config/retroarch.request";
pub const RA_BINARY: &str = "ra64.trimui";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RaRequest {
    Menu,
    Game {
        core: PathBuf,
        core_name: String,
        rom: PathBuf,
        link: Option<LinkRole>,
        options: Vec<(String, String)>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    RetroArch(PathBuf),
    Core(PathBuf),
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Missing::RetroArch(path) => write!(f, "no RetroArch at {}", path.display()),
            Missing::Core(path) => write!(f, "no core at {}", path.display()),
        }
    }
}

pub fn request_path(root: &Path) -> PathBuf {
    root.join(REQUEST_FILE)
}

fn retroarch_present(layout: &Layout) -> Result<(), Missing> {
    let binary = layout.retroarch().join(RA_BINARY);
    if binary.is_file() {
        Ok(())
    } else {
        Err(Missing::RetroArch(binary))
    }
}

pub fn menu_request(layout: &Layout) -> Result<RaRequest, Missing> {
    retroarch_present(layout)?;
    Ok(RaRequest::Menu)
}

pub fn game_request(
    layout: &Layout,
    cart: &Cart,
    core: Core,
    link: Option<(LinkRole, &str)>,
) -> Result<RaRequest, Missing> {
    retroarch_present(layout)?;
    let library = layout.app().join(dylib_name(core));
    if !library.is_file() {
        return Err(Missing::Core(library));
    }
    Ok(RaRequest::Game {
        core: library,
        core_name: core.text().to_string(),
        rom: cart.rom.clone(),
        link: link.map(|(role, _)| role),
        options: link.map_or_else(Vec::new, |(role, serial)| link_options(core, role, serial)),
    })
}

pub fn link_options(core: Core, role: LinkRole, serial: &str) -> Vec<(String, String)> {
    let pair = |key: &str, value: &str| (key.to_string(), value.to_string());
    match core {
        Core::Mgba => vec![
            pair("mgba_link", "on"),
            pair(
                "mgba_link_player",
                match role {
                    LinkRole::Host => "0",
                    LinkRole::Join => "1",
                },
            ),
        ],
        Core::Gpsp => vec![pair("gpsp_serial", serial)],
    }
}

impl RaRequest {
    pub fn to_text(&self) -> String {
        match self {
            RaRequest::Menu => "mode=menu\n".to_string(),
            RaRequest::Game {
                core,
                core_name,
                rom,
                link,
                options,
            } => {
                let mut text = format!(
                    "mode=game\ncore={}\ncore_name={core_name}\nrom={}\nlink={}\n",
                    core.display(),
                    rom.display(),
                    link.map_or("", LinkRole::arg)
                );
                for (key, value) in options {
                    text.push_str(&format!("option={key}={value}\n"));
                }
                text
            }
        }
    }

    pub fn parse(text: &str) -> Option<RaRequest> {
        let (mut mode, mut core, mut core_name, mut rom, mut link) = (None, None, None, None, None);
        let mut options = Vec::new();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "mode" => mode = Some(value),
                "core" => core = Some(PathBuf::from(value)),
                "core_name" => core_name = Some(value.to_string()),
                "rom" => rom = Some(PathBuf::from(value)),
                "link" => {
                    link = match value {
                        "host" => Some(LinkRole::Host),
                        "join" => Some(LinkRole::Join),
                        _ => None,
                    }
                }
                "option" => {
                    if let Some((k, v)) = value.split_once('=') {
                        options.push((k.to_string(), v.to_string()));
                    }
                }
                _ => {}
            }
        }
        let given = |p: Option<PathBuf>| p.filter(|p| !p.as_os_str().is_empty());
        match mode? {
            "menu" => Some(RaRequest::Menu),
            "game" => Some(RaRequest::Game {
                core: given(core)?,
                core_name: core_name.unwrap_or_default(),
                rom: given(rom)?,
                link,
                options,
            }),
            _ => None,
        }
    }

    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        atomic_write(&request_path(root), self.to_text().as_bytes())
    }

    pub fn read(root: &Path) -> Option<RaRequest> {
        std::fs::read_to_string(request_path(root))
            .ok()
            .and_then(|text| RaRequest::parse(&text))
    }
}

pub type RaRadio = Arc<dyn Fn(LinkRole, &Cancel) -> Result<(), RadioFail> + Send + Sync>;

pub fn ra_radio() -> RaRadio {
    Arc::new(crate::link_radio::up)
}

pub struct RaLinkUp {
    rx: Receiver<Result<(), RadioFail>>,
    cancel: Cancel,
}

impl RaLinkUp {
    pub fn spawn(radio: RaRadio, role: LinkRole) -> RaLinkUp {
        let (tx, rx) = channel();
        let cancel = Cancel::new();
        let flag = cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(radio(role, &flag));
        });
        RaLinkUp { rx, cancel }
    }

    pub fn poll(&mut self) -> Option<Result<(), RadioFail>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err(RadioFail::Radio("the radio thread went away".into())))
            }
        }
    }

    pub fn cancel(&mut self) {
        self.cancel.cancel();
    }
}

impl Drop for RaLinkUp {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
