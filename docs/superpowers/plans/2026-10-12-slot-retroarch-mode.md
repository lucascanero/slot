# Slot no Brick: modo "Use RetroArch" — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Uma opção "Use RetroArch" no Slot do Brick. Com ela, o Slot fica a prateleira e cada jogo abre no RetroArch do StockMix com o core escolhido no Slot. Inclui o link entre dois Bricks pelo netplay do RetroArch.

**Architecture:** A passagem do Slot para o RetroArch é feita pelo `launch.sh`.
- O Slot escreve um pedido (`Slot/Config/retroarch.request`) e sai como no "Exit to Menu".
- O `launch.sh` corre o RetroArch com o `retroarch.sh`, que reproduz o ambiente dos launchers do StockMix, e no fim reabre o Slot.
- O `keymon` volta a correr só enquanto o RetroArch está aberto.
- No link, o Slot levanta a rede direta (`slotlink.sh`) antes de sair. O pedido leva as opções de link do core, que o `retroarch.sh` põe num `.opt` do jogo só durante a sessão.

**Tech Stack:** Rust 1.96 (workspace existente, sem dependências novas), scripts POSIX `sh` (busybox no Brick), RetroArch 1.19.1 do StockMix (`ra64.trimui`), WSL Ubuntu 26.04 para build e testes.

**Spec:** [docs/superpowers/specs/2026-10-12-slot-retroarch-mode-design.md](../specs/2026-10-12-slot-retroarch-mode-design.md)

## Global Constraints

- Branch `brick`. Sem dependências novas no `Cargo.toml`. Finais de linha LF.
- Todos os comandos `cargo`, `sh card/...` e `ssh brick` correm no WSL Ubuntu-26.04, a partir de `/mnt/c/Users/lucas/Slot`. Os ajudantes estão no scratchpad: `run.sh` prepara o ambiente e `gate.sh` faz a gate completa.
  - O `gate.sh` corre fmt, clippy (host e `--features device`), todos os testes (com os 9 testes de base conhecidos saltados) e todos os `card/test-*.sh`.
- Caminhos no Brick:

  | O quê | Onde |
  |---|---|
  | Slot | `/mnt/SDCARD/Apps/Slot` (cores `mgba_libretro.so`, `gpsp_libretro.so` ao lado do `slot`) |
  | Dados do Slot | `/mnt/SDCARD/Slot` |
  | RetroArch | `/mnt/SDCARD/RetroArch/ra64.trimui`, com `HOME=/mnt/SDCARD/RetroArch/` |
  | Scripts do StockMix | `/mnt/SDCARD/System/usr/trimui/scripts` (`ra_audio_switcher.sh`, `cpufreq.sh`) |
  | Hotkeys do RetroArch | `/var/trimui_inputd/ra_hotkey` |

- O pedido: linhas `chave=valor`, nesta ordem e com estes nomes: `mode=game|menu`, `core=`, `core_name=` (`mGBA` ou `gpSP`, de `Core::text()`), `rom=`, `link=host|join|` (vazio sem link) e zero ou mais `option=chave=valor`.
- Opções de link:
  - mGBA: `mgba_link=on` e `mgba_link_player=0` (Host) ou `1` (Join);
  - gpSP: `gpsp_serial=<serial de link_mode(stem)>`.
- Endereço do anfitrião no link: `10.42.0.1`.
- Menu principal no modo Slot: Screen · Gameplay · Date & Time · About · Download Labels · Use RetroArch · Exit to Menu.
- Menu principal no modo RetroArch: Use RetroArch · RetroArch Settings · Start on Boot · Date & Time · About · Download Labels · Exit to Menu.
- Textos das linhas: "Use RetroArch" e "RetroArch Settings".
- Saves: os do Slot (`Slot/Saves`) e os do RetroArch nunca se tocam. Nunca escrever no `retroarch.cfg` do utilizador.
- Com "Use RetroArch" desligado, o Slot faz exatamente o que faz hoje.

## Review Focus

1. **Nomes de ROM com espaços, parênteses, `&`, apóstrofos e acentos** (`Pokémon - Emerald (U) [!] & Co's.gba`) chegam intactos ao `argv` do RetroArch. Testado na Tarefa 3 (ida e volta do pedido) e na Tarefa 8 (`retroarch.sh`).
2. **Um pedido velho no cartão** (falta de energia a meio) não abre o RetroArch sozinho no arranque seguinte. Testado na Tarefa 9.
3. **Falta de energia a meio de um link** deixa o `.opt` do jogo com `mgba_link = "on"`. A sessão seguinte tem de repor o ficheiro original do utilizador e não guardar o modificado como se fosse o original. Testado na Tarefa 8.
4. **Desligar "Use RetroArch" depois de uma passagem** não faz o Slot meter sozinho o cartucho lembrado no arranque seguinte. Testado na Tarefa 6.
5. **RetroArch Settings ou um cartucho sem RetroArch no cartão** são recusados, sem pedido e sem sair. Testado na Tarefa 6.

---

### Task 1: Verificação no Brick (spike, nada fica no código)

Responde a três perguntas antes de se escrever código:
- O `ra64.trimui` abre um jogo com um core do Slot?
- O netplay do RetroArch serve para o link?
- O que o `retroarch.sh` precisa para ficar igual a um jogo lançado pelo StockMix?

Instala também o build da suspensão (Tarefa 22 do plano do port), que ainda não chegou ao Brick.

**Files:**
- Modify: `docs/brick-hardware.md` (nova secção `## §16 RetroArch a partir do Slot`)

- [ ] **Step 1: Confirmar que o Brick responde**

Run: `ssh -o ConnectTimeout=5 brick true && echo up`
Expected: `up`.

Se não responder, pára aqui. Pede ao utilizador para ligar o Brick e o WiFi (IP 192.168.1.72) e espera pela resposta. Precisas do Brick e de alguém a olhar para o ecrã.

- [ ] **Step 2: Instalar o build atual (suspensão incluída)**

```bash
tar -C dist-brick -cf - Apps/Slot | ssh brick 'tar -C /mnt/SDCARD -xf - && chmod +x /mnt/SDCARD/Apps/Slot/slot /mnt/SDCARD/Apps/Slot/*.sh /mnt/SDCARD/Apps/Slot/boot/*.sh && sync && ls -la /mnt/SDCARD/Apps/Slot/suspend.sh /mnt/SDCARD/Apps/Slot/gpsp_libretro.so /mnt/SDCARD/Apps/Slot/mgba_libretro.so'
```
Expected: os três ficheiros listados.

- [ ] **Step 3: Recolher o que falta saber do StockMix**

```bash
ssh brick '
S=/mnt/SDCARD/System/usr/trimui/scripts
echo "== save_launcher"; cat $S/save_launcher.sh
echo "== ra_audio_switcher"; head -40 $S/ra_audio_switcher.sh
echo "== cfg"; grep -E "^(rgui_config_directory|game_specific_options|global_core_options|core_options_path|netplay_ip_port|netplay_use_mitm_server|netplay_check_frames|netplay_allow_slaves)" /mnt/SDCARD/RetroArch/.retroarch/retroarch.cfg
echo "== config dirs"; ls /mnt/SDCARD/RetroArch/.retroarch/config | head -20
echo "== keymon"; pid=$(pidof keymon); readlink /proc/$pid/exe; readlink /proc/$pid/cwd; tr "\000" " " < /proc/$pid/cmdline; echo; tr "\000" "\n" < /proc/$pid/environ | grep LD_LIBRARY_PATH
echo "== inputd"; ls -la /var/trimui_inputd/
echo "== roms"; ls /mnt/SDCARD/Roms/GBA | head -5
'
```
Expected: o conteúdo dos dois scripts e as chaves pedidas do `retroarch.cfg` (nenhuma delas é credencial; não acrescentes chaves `cloud_sync_*` ao grep). Também a linha do `keymon` e se o `ra_hotkey` já existe.

Anota no ledger:
- O que o `save_launcher.sh` faz. Se escreve algo de que o cloud sync ou o "resume" do StockMix dependa, é uma ruling: o `retroarch.sh` passa a fazê-lo também.
- O valor de `rgui_config_directory` e de `game_specific_options`. Com `game_specific_options = "false"`, o `.opt` do jogo não é lido e o link do mGBA não funciona; regista-o como ruling para a Tarefa 8.
- O `cmdline` do `keymon`. Se tiver argumentos além do nome, o relançamento da Tarefa 9 tem de os passar (ruling).

- [ ] **Step 4: Abrir um jogo GBA no RetroArch com o gpSP do Slot, pelo ciclo do StockMix**

Escolhe uma ROM da lista do Step 3 (`<ROM>` abaixo, caminho completo). O ciclo do `runtrimui.sh` corre o `/tmp/cmd_to_run.sh` com o `keymon` a correr, como num jogo normal:

```bash
ssh brick 'cat > /tmp/cmd_to_run.sh <<"EOF"
#!/bin/sh
export PATH="/mnt/SDCARD/System/usr/trimui/scripts:/mnt/SDCARD/System/bin:$PATH:/mnt/SDCARD/RetroArch"
export LD_LIBRARY_PATH="/usr/trimui/lib:/mnt/SDCARD/System/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
ra_audio_switcher.sh
touch /var/trimui_inputd/ra_hotkey
cpufreq.sh ondemand 2 6
cd /mnt/SDCARD/RetroArch
HOME=/mnt/SDCARD/RetroArch/ ./ra64.trimui -v -L /mnt/SDCARD/Apps/Slot/gpsp_libretro.so "<ROM>" > /mnt/SDCARD/Slot/ra-spike.log 2>&1
EOF
killall -9 MainUI'
```

Pede ao utilizador para:
1. ver se o jogo abre, com imagem e som;
2. testar A/B, D-pad e START;
3. carregar em MENU e ver se abre o menu do RetroArch;
4. mexer em Vol± e no brilho;
5. gravar no jogo (save in-game);
6. sair pelo menu do RetroArch (Quit).

Expected: o utilizador confirma os seis pontos e o Brick volta ao MainUI.

- [ ] **Step 5: Ler o log e os saves**

```bash
ssh brick '
grep -i -E "netpacket|loading core|libretro|savefile|cloud|error|fail" /mnt/SDCARD/Slot/ra-spike.log | head -40
ls -lt /mnt/SDCARD/RetroArch/.retroarch/saves/GBA | head -3
ls -la /var/trimui_inputd/
'
```
Expected:
- o log mostra o core do Slot carregado;
- há uma linha sobre a interface netpacket do gpSP;
- o `.srm` mais recente em `saves/GBA` é o do jogo;
- o `ra_hotkey` aparece na listagem ou não.

Anota no ledger se o `ra_hotkey` ficou depois de o RetroArch sair. Se ficou, a regra da spec (o `retroarch.sh` apaga-o se não existia) mantém-se. Se alguém já o apaga, regista-o, e a regra continua inofensiva.

- [ ] **Step 6: O mGBA do Slot com `--host` e o `.opt` do jogo**

O nome da pasta do core vem do `rgui_config_directory` do Step 3. Abaixo usa-se `<CFG>`; o valor por omissão é `/mnt/SDCARD/RetroArch/.retroarch/config`. `<STEM>` é o nome da ROM sem extensão.

```bash
ssh brick 'mkdir -p "<CFG>/mGBA" && printf "mgba_link = \"on\"\nmgba_link_player = \"0\"\n" > "<CFG>/mGBA/<STEM>.opt"
cat > /tmp/cmd_to_run.sh <<"EOF"
#!/bin/sh
export PATH="/mnt/SDCARD/System/usr/trimui/scripts:/mnt/SDCARD/System/bin:$PATH:/mnt/SDCARD/RetroArch"
export LD_LIBRARY_PATH="/usr/trimui/lib:/mnt/SDCARD/System/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
ra_audio_switcher.sh
touch /var/trimui_inputd/ra_hotkey
cd /mnt/SDCARD/RetroArch
HOME=/mnt/SDCARD/RetroArch/ ./ra64.trimui -v -L /mnt/SDCARD/Apps/Slot/mgba_libretro.so --host "<ROM>" > /mnt/SDCARD/Slot/ra-spike2.log 2>&1
EOF
killall -9 MainUI'
```
Pede ao utilizador que espere uns 10 segundos e saia pelo menu do RetroArch. Depois:

```bash
ssh brick 'grep -i -E "netplay|game-specific|core options|mgba_link|link" /mnt/SDCARD/Slot/ra-spike2.log | head -30; rm -f "<CFG>/mGBA/<STEM>.opt"'
```
Expected: o log mostra o netplay à escuta (por exemplo `[Netplay] ... waiting for client` ou equivalente) e a leitura das opções do jogo.

- [ ] **Step 7: Decidir e registar**

Escreve no fim de `docs/brick-hardware.md` uma secção `## §16 RetroArch a partir do Slot`, com o que se viu nos Steps 3–6. Usa o estilo das outras secções: frases curtas, listas e factos com os caminhos.

Regras:
- **Se o Step 4 falhou** (o jogo não abriu com o core do Slot): pára e conta ao utilizador antes de fazer outra tarefa. A spec pede isso.
- **Se não há sinal do netplay ou do netpacket** (Steps 5–6): fala com o utilizador. O link no modo RetroArch (Tarefa 7, e as opções da Tarefa 8) sai do plano por ruling e o resto continua.
- Pergunta ao utilizador se tem dois Bricks para o teste de link da Tarefa 10.

```bash
git add docs/brick-hardware.md
git commit -m "Survey how RetroArch runs Slot's cores on the Brick"
```

---

### Task 2: `use_retroarch` no `slot.state`

**Files:**
- Modify: `crates/slot-store/src/slot_state.rs`
- Test: `crates/slot-store/tests/durability.rs`

**Interfaces:**
- Produces: `SlotState::use_retroarch: bool` (falso por omissão), gravado como `use_retroarch=0|1`.

- [ ] **Step 1: Escrever os testes**

Acrescenta ao fim de `crates/slot-store/tests/durability.rs`:

```rust
#[test]
fn use_retroarch_round_trips_as_its_own_line() {
    let d = tmp_root();
    let s = SlotState {
        clock_set: true,
        use_retroarch: true,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    assert_eq!(read_slot_state(d.path()), s);
    let text = std::fs::read_to_string(d.path().join("Config/slot.state")).unwrap();
    assert!(
        text.lines().any(|l| l == "use_retroarch=1"),
        "no use_retroarch=1 in {text:?}"
    );
}

#[test]
fn a_card_from_before_retroarch_mode_reads_it_as_off() {
    let d = tmp_root();
    let s = SlotState {
        clock_set: true,
        brightness: 7,
        ..SlotState::default()
    };
    write_slot_state(d.path(), &s).unwrap();
    let path = d.path().join("Config/slot.state");
    let old: String = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("use_retroarch="))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&path, old).unwrap();
    let back = read_slot_state(d.path());
    assert!(!back.use_retroarch, "an old card came back in RetroArch mode");
    assert_eq!(back.brightness, 7, "the rest of an old card was thrown away");
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot-store --test durability use_retroarch`
Expected: erro de compilação `struct SlotState has no field named use_retroarch`.

- [ ] **Step 3: Implementar**

Em `crates/slot-store/src/slot_state.rs`:

1. Na `struct SlotState`, depois de `pub gb_palette: GbPalette,`:
```rust
    pub use_retroarch: bool,
```
2. No `Default`, depois de `gb_palette: GbPalette::DEFAULT,`:
```rust
            use_retroarch: false,
```
3. Em `write_slot_state`, acrescenta `use_retroarch={}\n` ao fim da string de formato, depois de `gb_palette={}\n`, e o argumento `s.use_retroarch as u8` depois de `s.gb_palette.core_name()` (com vírgula no anterior).
4. Em `parse`, junto às outras: `let mut use_retroarch = None;`. No `match key`, `"use_retroarch" => use_retroarch = flag(value),`. No `Some(SlotState { .. })`, `use_retroarch: use_retroarch.unwrap_or(fallback.use_retroarch),`.

- [ ] **Step 4: Ver passar**

Run: `cargo test -p slot-store --test durability`
Expected: todos passam, incluindo os dois novos.

- [ ] **Step 5: Commit**

```bash
git add crates/slot-store/src/slot_state.rs crates/slot-store/tests/durability.rs
git commit -m "Remember Use RetroArch in slot.state, off on cards from before it"
```

---

### Task 3: Onde está o RetroArch, e o pedido

**Files:**
- Modify: `crates/slot-store/src/layout.rs`
- Create: `crates/slot/src/retroarch.rs`
- Modify: `crates/slot/src/lib.rs` (`pub mod retroarch;`, por ordem alfabética, depois de `pub mod resample;`)
- Test: `crates/slot-store/tests/layout.rs`, `crates/slot/tests/retroarch.rs`

**Interfaces:**
- Consumes: `slot_store::{atomic_write, Cart, Core, Layout}`, `crate::core::dylib_name(Core) -> String`, `crate::link_net::Cancel`, `crate::link_radio::{LinkRole, RadioFail, up}`.
- Produces:
  - `Layout::retroarch(&self) -> &Path`: `<sd>/RetroArch` no StockMix, `<root>/RetroArch` no flat.
  - `slot::retroarch::{REQUEST_FILE, RA_BINARY, request_path(&Path) -> PathBuf}`
  - `slot::retroarch::RaRequest` (enum `Menu | Game { core: PathBuf, core_name: String, rom: PathBuf, link: Option<LinkRole>, options: Vec<(String, String)> }`), com `to_text(&self) -> String`, `parse(&str) -> Option<RaRequest>`, `write(&self, root: &Path) -> io::Result<()>` e `read(root: &Path) -> Option<RaRequest>`
  - `slot::retroarch::Missing` (`RetroArch(PathBuf) | Core(PathBuf)`, com `Display`)
  - `slot::retroarch::menu_request(&Layout) -> Result<RaRequest, Missing>`
  - `slot::retroarch::game_request(&Layout, &Cart, Core, Option<(LinkRole, &str)>) -> Result<RaRequest, Missing>`
  - `slot::retroarch::link_options(Core, LinkRole, serial: &str) -> Vec<(String, String)>`
  - `slot::retroarch::{RaRadio, ra_radio() -> RaRadio, RaLinkUp}`, com `RaLinkUp::spawn(RaRadio, LinkRole) -> RaLinkUp`, `poll(&mut self) -> Option<Result<(), RadioFail>>` e `cancel(&mut self)`

- [ ] **Step 1: Testes do `Layout`**

Acrescenta a `crates/slot-store/tests/layout.rs`:

```rust
#[test]
fn stockmix_finds_retroarch_at_the_root_of_the_card() {
    let l = Layout::stockmix(Path::new("/mnt/SDCARD"), Path::new("/mnt/SDCARD/Apps/Slot"));
    assert_eq!(l.retroarch(), Path::new("/mnt/SDCARD/RetroArch"));
}

#[test]
fn flat_keeps_a_retroarch_folder_in_its_root() {
    let l = Layout::flat(Path::new("/card"));
    assert_eq!(l.retroarch(), Path::new("/card/RetroArch"));
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot-store --test layout retroarch`
Expected: erro de compilação `no method named retroarch`.

- [ ] **Step 3: Implementar o `Layout`**

Em `crates/slot-store/src/layout.rs`:
- campo `retroarch: PathBuf,` na struct, depois de `boot_hook`;
- em `flat`: `retroarch: root.join("RetroArch"),`;
- em `stockmix`: `retroarch: sd.join("RetroArch"),`;
- o método, depois de `boot_script`:

```rust
    pub fn retroarch(&self) -> &Path {
        &self.retroarch
    }
```

Run: `cargo test -p slot-store --test layout`
Expected: todos passam.

- [ ] **Step 4: Testes do pedido**

Cria `crates/slot/tests/retroarch.rs`:

```rust
mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::tmp_root_with_carts;
use slot::core::dylib_name;
use slot::link_radio::{LinkRole, RadioFail};
use slot::retroarch::{
    game_request, link_options, menu_request, request_path, Missing, RaLinkUp, RaRequest,
    RA_BINARY,
};
use slot_store::{scan, Cart, Core, Layout};

fn install_retroarch(root: &Path) {
    let layout = Layout::flat(root);
    std::fs::create_dir_all(layout.retroarch()).unwrap();
    std::fs::write(layout.retroarch().join(RA_BINARY), b"#!/bin/sh\n").unwrap();
    std::fs::create_dir_all(layout.app()).unwrap();
    for core in Core::ALL {
        std::fs::write(layout.app().join(dylib_name(core)), b"").unwrap();
    }
}

fn first_cart(root: &Path) -> Cart {
    scan(root).unwrap().into_iter().next().expect("a cart")
}

#[test]
fn a_game_request_names_the_slot_core_beside_the_app_and_the_rom() {
    let d = tmp_root_with_carts(&["Emerald"]);
    install_retroarch(d.path());
    let layout = Layout::flat(d.path());
    let cart = first_cart(d.path());
    let r = game_request(&layout, &cart, Core::Gpsp, None).unwrap();
    assert_eq!(
        r,
        RaRequest::Game {
            core: layout.app().join(dylib_name(Core::Gpsp)),
            core_name: "gpSP".into(),
            rom: cart.rom.clone(),
            link: None,
            options: Vec::new(),
        }
    );
}

#[test]
fn the_request_is_the_text_launch_sh_reads() {
    let r = RaRequest::Game {
        core: "/a/mgba_libretro.so".into(),
        core_name: "mGBA".into(),
        rom: "/r/Emerald.gba".into(),
        link: Some(LinkRole::Join),
        options: vec![
            ("mgba_link".into(), "on".into()),
            ("mgba_link_player".into(), "1".into()),
        ],
    };
    assert_eq!(
        r.to_text(),
        "mode=game\ncore=/a/mgba_libretro.so\ncore_name=mGBA\nrom=/r/Emerald.gba\nlink=join\noption=mgba_link=on\noption=mgba_link_player=1\n"
    );
    assert_eq!(RaRequest::Menu.to_text(), "mode=menu\n");
    let plain = RaRequest::Game {
        core: "/a/gpsp_libretro.so".into(),
        core_name: "gpSP".into(),
        rom: "/r/Fusion.gba".into(),
        link: None,
        options: Vec::new(),
    };
    assert!(
        plain.to_text().lines().any(|l| l == "link="),
        "no empty link line: {:?}",
        plain.to_text()
    );
}

#[test]
fn a_request_round_trips_through_the_card_with_an_awkward_rom_name() {
    let d = tmp_root_with_carts(&[]);
    let r = RaRequest::Game {
        core: "/mnt/SDCARD/Apps/Slot/mgba_libretro.so".into(),
        core_name: "mGBA".into(),
        rom: "/mnt/SDCARD/Roms/GBA/Pokémon - Emerald (U) [!] & Co's = 2.gba".into(),
        link: Some(LinkRole::Host),
        options: vec![("gpsp_serial".into(), "mul_poke".into())],
    };
    r.write(d.path()).unwrap();
    assert!(request_path(d.path()).is_file());
    assert_eq!(RaRequest::read(d.path()), Some(r));
    RaRequest::Menu.write(d.path()).unwrap();
    assert_eq!(RaRequest::read(d.path()), Some(RaRequest::Menu));
}

#[test]
fn what_is_not_a_request_reads_as_none() {
    let d = tmp_root_with_carts(&[]);
    assert_eq!(RaRequest::read(d.path()), None, "no file at all");
    for text in [
        "",
        "garbage\n",
        "mode=bogus\n",
        "mode=game\nrom=/r/x.gba\n",
        "mode=game\ncore=/a/x.so\n",
        "mode=game\ncore=\nrom=\n",
    ] {
        assert_eq!(RaRequest::parse(text), None, "{text:?} read as a request");
    }
}

#[test]
fn mgba_links_by_player_and_gpsp_by_serial() {
    assert_eq!(
        link_options(Core::Mgba, LinkRole::Host, "auto"),
        vec![
            ("mgba_link".to_string(), "on".to_string()),
            ("mgba_link_player".to_string(), "0".to_string()),
        ]
    );
    assert_eq!(
        link_options(Core::Mgba, LinkRole::Join, "auto")[1],
        ("mgba_link_player".to_string(), "1".to_string())
    );
    assert_eq!(
        link_options(Core::Gpsp, LinkRole::Join, "rfu"),
        vec![("gpsp_serial".to_string(), "rfu".to_string())]
    );
}

#[test]
fn a_linked_game_request_carries_the_role_and_its_options() {
    let d = tmp_root_with_carts(&["Emerald"]);
    install_retroarch(d.path());
    let cart = first_cart(d.path());
    let r = game_request(
        &Layout::flat(d.path()),
        &cart,
        Core::Mgba,
        Some((LinkRole::Host, "auto")),
    )
    .unwrap();
    let RaRequest::Game { link, options, .. } = r else {
        panic!("not a game request");
    };
    assert_eq!(link, Some(LinkRole::Host));
    assert_eq!(options, link_options(Core::Mgba, LinkRole::Host, "auto"));
}

#[test]
fn with_no_retroarch_or_no_core_there_is_no_request() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let layout = Layout::flat(d.path());
    let cart = first_cart(d.path());
    let ra = layout.retroarch().join(RA_BINARY);
    assert_eq!(
        game_request(&layout, &cart, Core::Mgba, None),
        Err(Missing::RetroArch(ra.clone()))
    );
    assert_eq!(menu_request(&layout), Err(Missing::RetroArch(ra)));
    install_retroarch(d.path());
    let core = layout.app().join(dylib_name(Core::Gpsp));
    std::fs::remove_file(&core).unwrap();
    assert_eq!(
        game_request(&layout, &cart, Core::Gpsp, None),
        Err(Missing::Core(core))
    );
    assert_eq!(menu_request(&layout), Ok(RaRequest::Menu));
}

fn wait(up: &mut RaLinkUp) -> Result<(), RadioFail> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = up.poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "the radio never answered");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn the_radio_answer_comes_back_through_poll() {
    let mut up = RaLinkUp::spawn(
        Arc::new(|role, _| {
            assert_eq!(role, LinkRole::Join);
            Err(RadioFail::NoHost)
        }),
        LinkRole::Join,
    );
    assert_eq!(wait(&mut up), Err(RadioFail::NoHost));
}

#[test]
fn cancel_reaches_a_radio_that_is_still_coming_up() {
    let mut up = RaLinkUp::spawn(
        Arc::new(|_, cancel: &slot::link_net::Cancel| {
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(RadioFail::Cancelled)
        }),
        LinkRole::Host,
    );
    up.cancel();
    assert_eq!(wait(&mut up), Err(RadioFail::Cancelled));
}
```

- [ ] **Step 5: Ver falhar**

Run: `cargo test -p slot --test retroarch`
Expected: erro de compilação `unresolved import slot::retroarch`.

- [ ] **Step 6: Implementar `crates/slot/src/retroarch.rs`**

```rust
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
```

Se o `Arc::new(crate::link_radio::up)` não converter sozinho para `RaRadio` (lifetime de ordem superior do `&Cancel`), escreve `let radio: RaRadio = Arc::new(|role, cancel: &Cancel| crate::link_radio::up(role, cancel)); radio`.

- [ ] **Step 7: Ver passar**

Run: `cargo test -p slot --test retroarch` e `cargo clippy -p slot -q --no-default-features --features device -- -D warnings`
Expected: os 9 testes passam e o clippy do build do Brick não dá avisos.

- [ ] **Step 8: Commit**

```bash
git add crates/slot-store/src/layout.rs crates/slot-store/tests/layout.rs crates/slot/src/retroarch.rs crates/slot/src/lib.rs crates/slot/tests/retroarch.rs
git commit -m "Add the RetroArch hand-off request and where RetroArch lives on the card"
```

---

### Task 4: Menu — linhas e páginas por modo (`slot-ui`)

**Files:**
- Modify: `crates/slot-ui/src/quick_menu.rs`
- Modify: `crates/slot/src/app.rs` (só para compilar: braços novos nos `match` exaustivos e o campo novo do `QuickMenu`)
- Test: `crates/slot-ui/tests/quick_menu.rs`

**Interfaces:**
- Consumes: `SlotState::use_retroarch` (Tarefa 2).
- Produces:
  - `QuickRow::RetroArch` ("Use RetroArch") e `QuickRow::RetroArchSettings` ("RetroArch Settings"), no fim do enum e do `ALL` (que passa a ter 19);
  - `QuickRow::MAIN: [QuickRow; 7]` e `QuickRow::RETROARCH: [QuickRow; 7]`;
  - `QuickRow::top(retroarch: bool) -> QuickRow`;
  - `page_in`, `position_in`, `up_in`, `down_in`, `parent_in` (todas `(self, retroarch: bool)`). `page`, `position`, `up`, `down` e `parent` ficam como estão e chamam a versão `_in(false)`;
  - `QuickMenu::retroarch: bool`.

- [ ] **Step 1: Atualizar e escrever os testes**

Em `crates/slot-ui/tests/quick_menu.rs`, no teste `the_rows_run_in_the_order_the_user_chose`:
- a lista esperada de `QuickRow::MAIN.map(QuickRow::label)` passa a ser `["Screen", "Gameplay", "Date & Time", "About", "Download Labels", "Use RetroArch", "Exit to Menu"]`;
- a lista esperada de `opens` ganha `QuickRow::RetroArchSettings` no fim.

Acrescenta:

```rust
#[test]
fn the_retroarch_page_runs_in_its_own_order() {
    assert_eq!(
        QuickRow::RETROARCH.map(QuickRow::label),
        [
            "Use RetroArch",
            "RetroArch Settings",
            "Start on Boot",
            "Date & Time",
            "About",
            "Download Labels",
            "Exit to Menu"
        ]
    );
    for row in QuickRow::RETROARCH {
        assert_eq!(row.page_in(true), &QuickRow::RETROARCH[..], "{row:?}");
        assert_eq!(row.parent_in(true), None, "{row:?} has a parent page");
    }
    assert_eq!(QuickRow::top(true), QuickRow::RetroArch);
    assert_eq!(QuickRow::top(false), QuickRow::Screen);
}

#[test]
fn start_on_boot_is_on_gameplay_in_slot_mode_and_on_the_main_page_in_retroarch_mode() {
    assert_eq!(QuickRow::StartOnBoot.page_in(false), &QuickRow::GAME[..]);
    assert_eq!(QuickRow::StartOnBoot.parent_in(false), Some(QuickRow::Game));
    assert_eq!(QuickRow::StartOnBoot.page_in(true), &QuickRow::RETROARCH[..]);
    assert_eq!(QuickRow::StartOnBoot.position_in(true), 2);
}

#[test]
fn up_and_down_wrap_inside_the_page_of_the_mode() {
    assert_eq!(QuickRow::Exit.down_in(true), QuickRow::RetroArch);
    assert_eq!(QuickRow::RetroArch.up_in(true), QuickRow::Exit);
    assert_eq!(QuickRow::RetroArch.down_in(true), QuickRow::RetroArchSettings);
    assert_eq!(QuickRow::RetroArch.down_in(false), QuickRow::Exit);
    assert_eq!(QuickRow::Labels.down_in(false), QuickRow::RetroArch);
    assert_eq!(QuickRow::Exit.down_in(false), QuickRow::Screen);
}

#[test]
fn both_main_pages_stand_the_same_height_and_every_row_knows_its_index() {
    assert_eq!(QuickRow::MAIN.len(), QuickRow::RETROARCH.len());
    for (i, row) in QuickRow::ALL.iter().enumerate() {
        assert_eq!(row.index(), i, "{row:?}");
    }
    assert_eq!(QuickRow::RetroArch.label(), "Use RetroArch");
    assert!(!QuickRow::RetroArch.opens(), "Use RetroArch is an On/Off row");
    assert!(QuickRow::RetroArchSettings.opens());
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot-ui --test quick_menu`
Expected: erro de compilação (`no variant named RetroArch`, `no associated item named RETROARCH`).

- [ ] **Step 3: Implementar em `crates/slot-ui/src/quick_menu.rs`**

1. No enum, depois de `Labels,`: `RetroArch,` e `RetroArchSettings,`.
2. `ALL` passa a `[QuickRow; 19]`, com `QuickRow::RetroArch, QuickRow::RetroArchSettings,` no fim.
3. `MAIN` passa a:
```rust
    pub const MAIN: [QuickRow; 7] = [
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::Labels,
        QuickRow::RetroArch,
        QuickRow::Exit,
    ];

    pub const RETROARCH: [QuickRow; 7] = [
        QuickRow::RetroArch,
        QuickRow::RetroArchSettings,
        QuickRow::StartOnBoot,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::Labels,
        QuickRow::Exit,
    ];
```
4. Depois de `page()`:
```rust
    pub fn page_in(self, retroarch: bool) -> &'static [QuickRow] {
        if retroarch && QuickRow::RETROARCH.contains(&self) {
            &QuickRow::RETROARCH
        } else {
            self.page()
        }
    }

    pub fn top(retroarch: bool) -> QuickRow {
        if retroarch {
            QuickRow::RETROARCH[0]
        } else {
            QuickRow::MAIN[0]
        }
    }
```
5. `parent`, `position`, `up` e `down` passam a chamar as versões `_in`:
```rust
    pub fn parent(self) -> Option<QuickRow> {
        self.parent_in(false)
    }

    pub fn parent_in(self, retroarch: bool) -> Option<QuickRow> {
        match self.page_in(retroarch)[0] {
            QuickRow::GbaShader => Some(QuickRow::Screen),
            QuickRow::FastForward => Some(QuickRow::Game),
            _ => None,
        }
    }

    pub fn position(self) -> usize {
        self.position_in(false)
    }

    pub fn position_in(self, retroarch: bool) -> usize {
        self.page_in(retroarch)
            .iter()
            .position(|r| *r == self)
            .unwrap_or(0)
    }

    pub fn up(self) -> QuickRow {
        self.up_in(false)
    }

    pub fn up_in(self, retroarch: bool) -> QuickRow {
        let page = self.page_in(retroarch);
        page[(self.position_in(retroarch) + page.len() - 1) % page.len()]
    }

    pub fn down(self) -> QuickRow {
        self.down_in(false)
    }

    pub fn down_in(self, retroarch: bool) -> QuickRow {
        let page = self.page_in(retroarch);
        page[(self.position_in(retroarch) + 1) % page.len()]
    }
```
6. Em `label()`: `QuickRow::RetroArch => "Use RetroArch",` e `QuickRow::RetroArchSettings => "RetroArch Settings",`. Em `opens()`: acrescenta `| QuickRow::RetroArchSettings`.
7. `QuickMenu` ganha `pub retroarch: bool,` depois de `pub row: QuickRow,`. No `draw`, `for &row in self.row.page()` passa a `for &row in self.row.page_in(self.retroarch)`, e as duas chamadas `row_top(..)` passam a `row_top(.., self.retroarch)`.
8. `row_top`:
```rust
fn row_top(row: QuickRow, retroarch: bool) -> f32 {
    let page = row.page_in(retroarch);
    let top = (OUT_H as f32 - QUICK_PITCH * page.len() as f32) / 2.0;
    top + QUICK_PITCH * row.position_in(retroarch) as f32
}
```

- [ ] **Step 4: Fazer o `app.rs` compilar**

Em `crates/slot/src/app.rs`:
- em `quick_value`: `QuickRow::RetroArch => Some(QuickValue::flag(self.state.use_retroarch)),` e `QuickRow::RetroArchSettings` junto aos que devolvem `None`;
- em `open_quick_row` e `change_setting`: `QuickRow::RetroArch | QuickRow::RetroArchSettings` junto aos braços que não fazem nada (`=> {}` em `open_quick_row` e `=> return` em `change_setting`);
- na construção do `QuickMenu` (`Phase::QuickMenu { row } => QuickMenu {`): `retroarch: self.state.use_retroarch,`.

- [ ] **Step 5: Ver passar**

Run: `cargo test -p slot-ui --test quick_menu` e depois `cargo test -p slot --test quick_menu`
Expected: tudo passa. Os testes do `slot` andam pela página principal com Down e continuam certos com 7 linhas.

- [ ] **Step 6: Commit**

```bash
git add crates/slot-ui/src/quick_menu.rs crates/slot-ui/tests/quick_menu.rs crates/slot/src/app.rs
git commit -m "Add Use RetroArch and RetroArch Settings rows, and a main page per mode"
```

---

### Task 5: App — "Use RetroArch" no menu

**Files:**
- Modify: `crates/slot/src/app.rs`
- Test: `crates/slot/tests/retroarch_mode.rs` (novo)

**Interfaces:**
- Consumes: `QuickRow::{top, up_in, down_in, parent_in, RetroArch}` (Tarefa 4), `SlotState::use_retroarch` (Tarefa 2).
- Produces: `App::toggle_retroarch` (privada). Ao desligar o modo, esquece `state.cart` e `state.cart_platform`.

- [ ] **Step 1: Escrever os testes**

Cria `crates/slot/tests/retroarch_mode.rs`:

```rust
mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, Clock, CLOCK_IS_SET};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, Platform, SlotState};
use slot_ui::{QuickRow, QuickValue};
use tempfile::TempDir;

fn booted_with(d: &Path, state: SlotState) -> (App, Clock) {
    write_slot_state(
        d,
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    app_booting_at(d, CLOCK_IS_SET)
}

fn on_carousel(use_retroarch: bool) -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (a, clock) = booted_with(
        d.path(),
        SlotState {
            use_retroarch,
            ..SlotState::default()
        },
    );
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "not on the carousel: {:?}",
        a.phase()
    );
    (d, a, clock)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn walk_to(a: &mut App, row: QuickRow) {
    if a.quick_menu().is_none() {
        a.apply(Action::QuickMenu);
    }
    for _ in 0..=QuickRow::MAIN.len() {
        if a.quick_menu() == Some(row) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("the bar never reached {row:?}");
}

#[test]
fn use_retroarch_stands_above_exit_and_starts_off() {
    let (_d, mut a, _) = on_carousel(false);
    walk_to(&mut a, QuickRow::RetroArch);
    assert_eq!(a.quick_value(QuickRow::RetroArch), Some(QuickValue::Off));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Exit));
}

#[test]
fn turning_it_on_saves_and_swaps_screen_and_gameplay_for_retroarch_settings() {
    let (d, mut a, _) = on_carousel(false);
    walk_to(&mut a, QuickRow::RetroArch);
    press(&mut a, Btn::Right);
    assert!(read_slot_state(d.path()).use_retroarch, "never reached the card");
    assert_eq!(a.quick_value(QuickRow::RetroArch), Some(QuickValue::On));
    assert_eq!(
        a.quick_menu(),
        Some(QuickRow::RetroArch),
        "the bar left the row it changed"
    );
    let mut seen = Vec::new();
    for _ in 0..QuickRow::RETROARCH.len() {
        seen.push(a.quick_menu().unwrap());
        press(&mut a, Btn::Down);
    }
    assert_eq!(seen, QuickRow::RETROARCH.to_vec());
}

#[test]
fn the_menu_opens_on_use_retroarch_in_retroarch_mode() {
    let (_d, mut a, _) = on_carousel(true);
    a.apply(Action::QuickMenu);
    assert_eq!(a.quick_menu(), Some(QuickRow::RetroArch));
}

#[test]
fn b_on_start_on_boot_in_retroarch_mode_closes_the_menu() {
    let (_d, mut a, _) = on_carousel(true);
    walk_to(&mut a, QuickRow::StartOnBoot);
    press(&mut a, Btn::B);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "B went to a Gameplay page RetroArch mode does not show: {:?}",
        a.phase()
    );
}

#[test]
fn turning_it_off_brings_the_slot_pages_back_and_forgets_the_remembered_cart() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (mut a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            cart: Some("Fusion".into()),
            cart_platform: Some(Platform::Gba),
            ..SlotState::default()
        },
    );
    walk_to(&mut a, QuickRow::RetroArch);
    press(&mut a, Btn::Left);
    let s = read_slot_state(d.path());
    assert!(!s.use_retroarch);
    assert_eq!(s.cart, None, "Slot mode would insert the remembered cart on boot");
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Exit));
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot --test retroarch_mode`
Expected: falham `turning_it_on_...` (o Right não muda nada), `the_menu_opens_on_use_retroarch_...` (abre em `Screen`), `b_on_start_on_boot_...` (o `walk_to` não encontra `StartOnBoot` na página principal) e `turning_it_off_...`. `use_retroarch_stands_above_exit_and_starts_off` já passa por causa da Tarefa 4; isso está certo.

- [ ] **Step 3: Implementar em `crates/slot/src/app.rs`**

1. `open_quick_menu`: `row: QuickRow::top(self.state.use_retroarch),`.
2. `quick_menu_input`, no início: `let retroarch = self.state.use_retroarch;`. Depois `row.up()` → `row.up_in(retroarch)`, `row.down()` → `row.down_in(retroarch)` e `row.parent()` → `row.parent_in(retroarch)`.
3. `change_setting`, logo a seguir ao `if row == QuickRow::StartOnBoot { .. }`:
```rust
        if row == QuickRow::RetroArch {
            return self.toggle_retroarch();
        }
```
4. Nova função, depois de `toggle_start_on_boot`:
```rust
    fn toggle_retroarch(&mut self) {
        self.state.use_retroarch = !self.state.use_retroarch;
        if !self.state.use_retroarch {
            self.state.cart = None;
            self.state.cart_platform = None;
        }
        self.persist();
    }
```

- [ ] **Step 4: Ver passar**

Run: `cargo test -p slot --test retroarch_mode` e `cargo test -p slot --test quick_menu`
Expected: tudo passa.

- [ ] **Step 5: Commit**

```bash
git add crates/slot/src/app.rs crates/slot/tests/retroarch_mode.rs
git commit -m "Toggle Use RetroArch from the quick menu, with the main page following the mode"
```

---

### Task 6: App — passar o jogo ao RetroArch e voltar

**Files:**
- Modify: `crates/slot/src/app.rs`
- Test: `crates/slot/tests/retroarch_mode.rs`

**Interfaces:**
- Consumes: `slot::retroarch::{game_request, menu_request, RaRequest}` (Tarefa 3), `slot_store::{core_for_platform, Layout}`.
- Produces (privadas, usadas pela Tarefa 7):
  - `App::front_request(&mut self, link: Option<LinkRole>) -> Option<RaRequest>` recusa e regista no log quando falta algo;
  - `App::hand_off(&mut self, request: RaRequest) -> bool`, `false` se o pedido não ficou escrito.

- [ ] **Step 1: Escrever os testes**

Acrescenta a `crates/slot/tests/retroarch_mode.rs`:
- aos `use` do topo: `use slot::core::dylib_name;`, `use slot::retroarch::{RaRequest, RA_BINARY};` e `use slot_store::{write_selected_core, Core, Layout};`;
- o resto:

```rust
fn install_retroarch(root: &Path) {
    let layout = Layout::flat(root);
    std::fs::create_dir_all(layout.retroarch()).unwrap();
    std::fs::write(layout.retroarch().join(RA_BINARY), b"#!/bin/sh\n").unwrap();
    std::fs::create_dir_all(layout.app()).unwrap();
    for core in Core::ALL {
        std::fs::write(layout.app().join(dylib_name(core)), b"").unwrap();
    }
}

fn game(root: &Path) -> (std::path::PathBuf, String, std::path::PathBuf) {
    match RaRequest::read(root) {
        Some(RaRequest::Game {
            core,
            core_name,
            rom,
            ..
        }) => (core, core_name, rom),
        other => panic!("no game request on the card: {other:?}"),
    }
}

#[test]
fn a_cart_put_in_in_retroarch_mode_goes_to_retroarch_and_slot_leaves() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(a.exiting(), "slot did not leave for RetroArch");
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "slot started the game itself: {:?}",
        a.phase()
    );
    let (core, name, rom) = game(d.path());
    assert_eq!(core, Layout::flat(d.path()).app().join(dylib_name(Core::Mgba)));
    assert_eq!(name, "mGBA");
    assert_eq!(rom, d.path().join("Games/GBA/Emerald.gba"));
    assert_eq!(read_slot_state(d.path()).cart.as_deref(), Some("Emerald"));
}

#[test]
fn retroarch_loads_the_core_chosen_on_the_cart() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.apply(Action::Insert);
    let (core, name, _) = game(d.path());
    assert_eq!(core, Layout::flat(d.path()).app().join(dylib_name(Core::Gpsp)));
    assert_eq!(name, "gpSP");
}

#[test]
fn a_game_boy_cart_goes_to_mgba_whatever_was_written_for_it() {
    let d = common::tmp_root_with_gb_carts(&["Tetris"]);
    install_retroarch(d.path());
    write_selected_core(d.path(), "Tetris", Core::Gpsp).unwrap();
    let (mut a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            ..SlotState::default()
        },
    );
    a.apply(Action::Insert);
    assert_eq!(game(d.path()).1, "mGBA");
}

#[test]
fn with_no_retroarch_on_the_card_the_cart_is_refused_and_slot_stays() {
    let (d, mut a, _) = on_carousel(true);
    a.apply(Action::Insert);
    assert!(!a.exiting(), "slot left for a RetroArch that is not there");
    assert!(a.refusal_active(a.now()), "no refusal");
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn with_the_core_missing_the_cart_is_refused() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    std::fs::remove_file(Layout::flat(d.path()).app().join(dylib_name(Core::Mgba))).unwrap();
    a.apply(Action::Insert);
    assert!(!a.exiting());
    assert!(a.refusal_active(a.now()));
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn retroarch_settings_opens_retroarch_with_no_game() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    walk_to(&mut a, QuickRow::RetroArchSettings);
    press(&mut a, Btn::A);
    assert!(a.exiting(), "A on RetroArch Settings did not leave");
    assert_eq!(RaRequest::read(d.path()), Some(RaRequest::Menu));
}

#[test]
fn retroarch_settings_without_retroarch_is_refused() {
    let (d, mut a, _) = on_carousel(true);
    walk_to(&mut a, QuickRow::RetroArchSettings);
    press(&mut a, Btn::A);
    assert!(!a.exiting());
    assert!(a.refusal_active(a.now()));
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn coming_back_from_retroarch_shows_the_shelf_on_that_cart_without_putting_it_in() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let (a, _) = booted_with(
        d.path(),
        SlotState {
            use_retroarch: true,
            cart: Some("Fusion".into()),
            cart_platform: Some(Platform::Gba),
            ..SlotState::default()
        },
    );
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "slot put the cart in on its own: {:?}",
        a.phase()
    );
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert!(!a.exiting(), "slot went straight back to RetroArch");
}

#[test]
fn turning_retroarch_off_after_a_hand_off_does_not_put_the_cart_in_next_boot() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(a.exiting());
    std::fs::remove_file(slot::retroarch::request_path(d.path())).unwrap();
    let (mut back, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    walk_to(&mut back, QuickRow::RetroArch);
    press(&mut back, Btn::Left);
    let (later, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(
        matches!(later.phase(), Phase::Shelf),
        "Slot mode put RetroArch's last cart in: {:?}",
        later.phase()
    );
}

#[test]
fn slot_mode_still_plays_the_cart_itself() {
    let (d, mut a, _) = on_carousel(false);
    install_retroarch(d.path());
    a.apply(Action::Insert);
    assert!(matches!(a.phase(), Phase::Inserting { .. }), "{:?}", a.phase());
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot --test retroarch_mode`
Expected:
- falham todos os novos exceto `slot_mode_still_plays_the_cart_itself`;
- os de inserir mostram `Phase::Inserting`;
- os do RetroArch Settings não saem;
- `coming_back_...` mostra `Inserting`.

- [ ] **Step 3: Implementar em `crates/slot/src/app.rs`**

1. Imports: `use crate::retroarch::RaRequest;`.
2. `start()`, no início:
```rust
        if self.state.use_retroarch {
            return self.start_on_the_shelf();
        }
```
   e logo a seguir a `start()`:
```rust
    fn start_on_the_shelf(&mut self) {
        self.phase = Phase::Shelf;
        let platform = self.state.cart_platform;
        let front = self
            .state
            .cart
            .clone()
            .and_then(|stem| self.seat_of(&stem, platform));
        if let Some((at, i)) = front {
            self.shelf_at = at;
            self.shelf_mut().select(i);
        }
        self.name_pending = true;
    }
```
3. `insert()`, logo depois de `if !self.on_shelf() { return; }`:
```rust
        if self.state.use_retroarch {
            self.play_held = None;
            if let Some(request) = self.front_request(None) {
                self.hand_off(request);
            }
            return;
        }
```
4. `open_quick_row`: tira `QuickRow::RetroArchSettings` do braço vazio e acrescenta `QuickRow::RetroArchSettings => self.hand_off_menu(),`.
5. Novas funções, depois de `begin_exit`:
```rust
    fn front_request(&mut self, link: Option<LinkRole>) -> Option<RaRequest> {
        let root = self.root.clone()?;
        let at = self.shelf().index;
        let cart = self.shelf_mut().carts.get_mut(at).map(|c| {
            c.read_header();
            c.clone()
        })?;
        let core = slot_store::core_for_platform(&root, &cart.stem, cart.platform);
        let serial = self.link_mode(&cart.stem).1;
        let layout = slot_store::Layout::for_root(&root);
        match crate::retroarch::game_request(&layout, &cart, core, link.map(|role| (role, serial)))
        {
            Ok(request) => Some(request),
            Err(missing) => {
                eprintln!("slot: retroarch: {}: {missing}", cart.stem);
                self.refuse();
                None
            }
        }
    }

    fn hand_off_menu(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        match crate::retroarch::menu_request(&slot_store::Layout::for_root(&root)) {
            Ok(request) => {
                self.hand_off(request);
            }
            Err(missing) => {
                eprintln!("slot: retroarch: {missing}");
                self.refuse();
            }
        }
    }

    fn hand_off(&mut self, request: RaRequest) -> bool {
        let Some(root) = self.root.clone() else {
            return false;
        };
        if let Err(e) = request.write(&root) {
            eprintln!("slot: retroarch: could not write the request: {e}");
            self.card_failing.store(true, Ordering::Relaxed);
            self.refuse();
            return false;
        }
        eprintln!("slot: retroarch: handing over to RetroArch");
        self.remember_front();
        self.begin_exit();
        true
    }

    fn remember_front(&mut self) {
        let front = self
            .shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| (c.stem.clone(), c.platform));
        let Some((stem, platform)) = front else {
            return;
        };
        self.state.cart = Some(stem);
        self.state.cart_platform = Some(platform);
        self.persist();
    }
```

- [ ] **Step 4: Ver passar, e ver que nada antigo partiu**

Run: `cargo test -p slot --test retroarch_mode`, depois `cargo test -p slot --test boot --test app --test eject --test quick_menu`
Expected: tudo passa. O modo Slot não mudou: o caminho novo só corre com `use_retroarch`.

- [ ] **Step 5: Commit**

```bash
git add crates/slot/src/app.rs crates/slot/tests/retroarch_mode.rs
git commit -m "In RetroArch mode, hand the cart or the RetroArch menu over and come back to the shelf"
```

---

### Task 7: App — link no modo RetroArch

**Files:**
- Modify: `crates/slot/src/app.rs`
- Test: `crates/slot/tests/retroarch_mode.rs`

**Interfaces:**
- Consumes: `front_request`, `hand_off` (Tarefa 6); `RaLinkUp`, `RaRadio`, `ra_radio` (Tarefa 3); `GameMenu`, `LinkRow`, `LinkStep`, `LinkFail` e `RadioJob` (existentes).
- Produces: `App::set_ra_radio(&mut self, radio: RaRadio)` (pública, para os testes).

- [ ] **Step 1: Escrever os testes**

Acrescenta a `crates/slot/tests/retroarch_mode.rs`:
- aos `use` do topo: `use std::sync::atomic::{AtomicBool, Ordering};`, `use std::sync::Arc;`, `use std::time::{Duration, Instant};`, `use slot::app::GameMenu;`, `use slot::link_radio::{LinkRole, RadioFail};`, `use slot::link_start::LinkFail;` e `use slot_ui::Toast;`;
- o resto:

```rust
fn settle(a: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while matches!(a.game_menu(), Some(GameMenu::Working { .. })) {
        a.update(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(1));
        assert!(Instant::now() < deadline, "the overlay never left Working");
    }
}

fn linked(d: &Path) -> (Option<LinkRole>, Vec<(String, String)>) {
    match RaRequest::read(d) {
        Some(RaRequest::Game { link, options, .. }) => (link, options),
        other => panic!("no game request on the card: {other:?}"),
    }
}

#[test]
fn select_on_a_cart_opens_host_and_join_only_in_retroarch_mode() {
    let (_d, mut a, _) = on_carousel(true);
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), Some(GameMenu::Pick(slot::app::LinkRow::Host)));
    let (_d, mut slot_mode, _) = on_carousel(false);
    slot_mode.apply(Action::GbaDown(Btn::Select));
    assert_eq!(slot_mode.game_menu(), None, "Slot mode's shelf changed");
}

#[test]
fn host_brings_the_radio_up_then_hands_off_with_the_link() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|role, _| {
        assert_eq!(role, LinkRole::Host);
        Ok(())
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    assert!(matches!(a.game_menu(), Some(GameMenu::Working { .. })));
    settle(&mut a);
    assert!(a.exiting(), "slot never left for RetroArch");
    assert_eq!(a.game_menu(), None);
    let (link, options) = linked(d.path());
    assert_eq!(link, Some(LinkRole::Host));
    assert_eq!(
        options,
        slot::retroarch::link_options(Core::Mgba, LinkRole::Host, "auto")
    );
}

#[test]
fn join_is_player_one() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, _| Ok(())));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::Right);
    press(&mut a, Btn::A);
    settle(&mut a);
    let (link, options) = linked(d.path());
    assert_eq!(link, Some(LinkRole::Join));
    assert!(
        options.contains(&("mgba_link_player".to_string(), "1".to_string())),
        "{options:?}"
    );
}

#[test]
fn a_join_with_no_host_shows_the_failure_and_slot_stays() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, _| Err(RadioFail::NoHost)));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::Right);
    press(&mut a, Btn::A);
    settle(&mut a);
    assert!(
        matches!(
            a.game_menu(),
            Some(GameMenu::Failed {
                fail: LinkFail::NobodyCame,
                ..
            })
        ),
        "{:?}",
        a.game_menu()
    );
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
    press(&mut a, Btn::A);
    assert_eq!(a.game_menu(), None, "OK did not close the failure");
}

#[test]
fn b_while_the_radio_comes_up_cancels_without_handing_off() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    a.set_ra_radio(Arc::new(|_, cancel: &slot::link_net::Cancel| {
        while !cancel.is_cancelled() {
            std::thread::sleep(Duration::from_millis(2));
        }
        Err(RadioFail::Cancelled)
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    press(&mut a, Btn::B);
    settle(&mut a);
    assert_eq!(a.game_menu(), None);
    assert!(!a.exiting());
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn with_retroarch_missing_a_link_is_refused_before_the_radio_comes_up() {
    let (d, mut a, _) = on_carousel(true);
    let asked = Arc::new(AtomicBool::new(false));
    let flag = asked.clone();
    a.set_ra_radio(Arc::new(move |_, _| {
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    std::thread::sleep(Duration::from_millis(20));
    a.update(1.0 / 60.0);
    assert_eq!(a.game_menu(), None);
    assert!(a.refusal_active(a.now()));
    assert!(!asked.load(Ordering::SeqCst), "the radio came up for nothing");
    assert_eq!(RaRequest::read(d.path()), None);
}

#[test]
fn gpsp_on_a_cart_with_no_link_says_so() {
    let (d, mut a, _) = on_carousel(true);
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), None);
    assert_eq!(a.toast(), Some(Toast::NoLink));
}

#[test]
fn mgba_on_a_wireless_adapter_cart_says_to_use_gpsp() {
    let (d, mut a, _) = on_carousel(true);
    common::write_retail_header(&d, "Emerald", "POKEMON EMER", "BPEE");
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.game_menu(), None);
    assert_eq!(a.toast(), Some(Toast::NeedsGpsp));
}

#[test]
fn gpsp_on_a_link_cart_carries_the_serial_slot_would_use() {
    let (d, mut a, _) = on_carousel(true);
    install_retroarch(d.path());
    common::write_retail_header(&d, "Emerald", "POKEMON EMER", "BPEE");
    write_selected_core(d.path(), "Emerald", Core::Gpsp).unwrap();
    a.set_ra_radio(Arc::new(|_, _| Ok(())));
    a.apply(Action::GbaDown(Btn::Select));
    press(&mut a, Btn::A);
    settle(&mut a);
    let (_, options) = linked(d.path());
    let want = a.link_mode("Emerald").1;
    assert_eq!(options, vec![("gpsp_serial".to_string(), want.to_string())]);
}
```

- [ ] **Step 2: Ver falhar**

Run: `cargo test -p slot --test retroarch_mode`
Expected: erro de compilação `no method named set_ra_radio`. Quando o método existir mas ainda vazio, os testes de link falham porque o SELECT não abre nada.

- [ ] **Step 3: Implementar em `crates/slot/src/app.rs`**

1. Imports: `use crate::link_radio::RadioFail;` (junta ao `use` existente de `link_radio`) e `use crate::retroarch::{ra_radio, RaLinkUp, RaRadio, RaRequest};` (substitui o `use crate::retroarch::RaRequest;` da Tarefa 6).
2. Campos da `App`, depois de `starting: Option<LinkStarting>,`:
```rust
    ra_link: Option<(RaLinkUp, RaRequest)>,
    ra_radio: RaRadio,
```
   e em `App::new`, depois de `starting: None,`: `ra_link: None,` e `ra_radio: ra_radio(),`.
3. Na prateleira (`Phase::Shelf => match action {`), logo depois do braço `_ if self.core_picker.is_some() => self.core_picker_input(action),`:
```rust
                Action::GbaDown(Btn::Select) if self.state.use_retroarch => {
                    self.open_ra_link_menu()
                }
```
4. `pick_link`, no início:
```rust
        if self.state.use_retroarch {
            return self.ra_pick_link(role);
        }
```
5. `game_menu_input`, no braço `GameMenu::Working { .. }`, dentro do `if action == Action::GbaDown(Btn::B) {`, junta:
```rust
                    if let Some((up, _)) = &mut self.ra_link {
                        up.cancel();
                    }
```
6. `close_game_menu`, depois do `if let Some(mut starting) = self.starting.take() { .. }`:
```rust
        if let Some((mut up, _)) = self.ra_link.take() {
            up.cancel();
            self.radio.ask(RadioJob::Down);
        }
```
7. `update`, logo a seguir a `self.poll_link();`: `self.poll_ra_link();`.
8. Novas funções, depois de `pick_link`:
```rust
    pub fn set_ra_radio(&mut self, radio: RaRadio) {
        self.ra_radio = radio;
    }

    fn open_ra_link_menu(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let at = self.shelf().index;
        let Some(cart) = self.shelf_mut().carts.get_mut(at).map(|c| {
            c.read_header();
            c.clone()
        }) else {
            return;
        };
        let core = slot_store::core_for_platform(&root, &cart.stem, cart.platform);
        let (kind, _) = self.link_mode(&cart.stem);
        if core == Core::Mgba {
            if kind == LinkKind::Wireless {
                self.hud.toast(Toast::NeedsGpsp, self.now());
                return;
            }
            self.link_hardware = LinkKind::Cable;
        } else {
            if !link_carried(&cart.code, &cart.title) {
                self.hud.toast(Toast::NoLink, self.now());
                return;
            }
            self.link_hardware = kind;
        }
        self.shelf_mut().release_hold();
        self.play_held = None;
        self.radio.ask(RadioJob::Warm);
        self.game_menu = Some(GameMenu::Pick(self.last_role));
    }

    fn ra_pick_link(&mut self, role: LinkRow) {
        let Some(request) = self.front_request(Some(role.role())) else {
            return self.close_game_menu();
        };
        let up = RaLinkUp::spawn(self.ra_radio.clone(), role.role());
        self.ra_link = Some((up, request));
        self.game_menu = Some(GameMenu::Working {
            role,
            step: LinkStep::Radio,
            since: self.now(),
        });
    }

    fn poll_ra_link(&mut self) {
        let Some((up, _)) = &mut self.ra_link else {
            return;
        };
        let Some(result) = up.poll() else {
            return;
        };
        let Some((_, request)) = self.ra_link.take() else {
            return;
        };
        let (role, worked) = match self.game_menu {
            Some(GameMenu::Working { role, since, .. }) => (role, since),
            _ => (self.last_role, self.now()),
        };
        let fail = match result {
            Ok(()) => {
                self.game_menu = None;
                if !self.hand_off(request) {
                    self.radio.ask(RadioJob::Down);
                }
                return;
            }
            Err(RadioFail::Cancelled) => {
                self.radio.ask(RadioJob::Down);
                self.game_menu = None;
                return;
            }
            Err(RadioFail::NoHost) => LinkFail::NobodyCame,
            Err(RadioFail::Radio(why)) => {
                eprintln!("slot: retroarch: link radio: {why}");
                LinkFail::Radio
            }
        };
        self.radio.ask(RadioJob::Down);
        self.game_menu = Some(GameMenu::Failed {
            role,
            fail,
            worked,
            since: self.now(),
        });
    }
```
   Repara que, no `Ok`, o menu fecha com `self.game_menu = None` e **não** com `close_game_menu()`. A rede direta tem de continuar de pé até o `launch.sh` a desfazer depois do RetroArch.

- [ ] **Step 4: Ver passar, e ver que o link do modo Slot não mudou**

Run: `cargo test -p slot --test retroarch_mode`, depois `cargo test -p slot --test link_ui --test link_session -- --skip a_in_the_mode_the_game_already_runs_starts_the_link_straight_away --skip a_link_in_a_switched_mode_reloads_the_game_and_then_starts_the_link --skip a_reloads_only_for_a_serial_the_core_was_not_loaded_with --skip a_switch_is_refused_over_a_resume_the_core_would_not_take --skip select_is_refused_where_gpsp_would_link_the_same_either_way --skip the_reload_finishing_starts_the_link`
Expected: tudo passa. Os `--skip` são falhas de base conhecidas (ver `gate.sh`).

- [ ] **Step 5: Commit**

```bash
git add crates/slot/src/app.rs crates/slot/tests/retroarch_mode.rs
git commit -m "Link in RetroArch mode: SELECT picks Host or Join, the radio comes up, then RetroArch takes over"
```

---

### Task 8: `retroarch.sh`

**Files:**
- Create: `card/Apps/Slot/retroarch.sh`
- Create: `card/test-retroarch.sh`
- Modify: `taskfile.yml` (`test:card`: `- sh card/test-retroarch.sh`)

**Interfaces:**
- Consumes: o ficheiro do pedido (formato das Global Constraints), passado como `$1`.
- Produces: `sh retroarch.sh <ficheiro do pedido>`.
  - Sai com o código do RetroArch, ou com 2 se o pedido não serve ou o RetroArch não existe (nesse caso o RetroArch não corre).
  - Variáveis para os testes: `SLOT_SD`, `SLOT_RA_DIR`, `SLOT_RA_SCRIPTS`, `SLOT_RA_HOTKEY`, `SLOT_LINK_HOST`.

- [ ] **Step 1: Escrever o teste**

Cria `card/test-retroarch.sh`:

```sh
#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/Apps/Slot/retroarch.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

RA="$TMP/sd/RetroArch"
CFG="$RA/.retroarch/config"
GAME="Pokémon - Emerald (U) [!] & Co's"
ROM="$TMP/sd/Roms/GBA/$GAME.gba"
CORE="$TMP/app/mgba_libretro.so"

setup() {
	rm -rf "$TMP/sd" "$TMP/app" "$TMP"/*.log "$TMP/ra_hotkey"
	mkdir -p "$RA/.retroarch" "$TMP/sd/Roms/GBA" "$TMP/app" "$TMP/sd/System/usr/trimui/scripts"
	: > "$ROM"
	: > "$CORE"
	echo 'rgui_config_directory = "~/.retroarch/config"' > "$RA/.retroarch/retroarch.cfg"
	cat > "$RA/ra64.trimui" <<-SH
		#!/bin/sh
		{
			printf 'args='; printf '%s|' "\$@"; echo
			echo "home=\$HOME pwd=\$(pwd)"
			echo "lib=\$LD_LIBRARY_PATH"
			[ -f "$TMP/ra_hotkey" ] && echo "hotkey=yes" || echo "hotkey=no"
			cat "$CFG/mGBA/$GAME.opt" 2>/dev/null | sed 's/^/opt: /'
		} >> "$TMP/ra.log"
		exit \${RA_EXIT:-0}
	SH
	for s in ra_audio_switcher.sh cpufreq.sh; do
		cat > "$TMP/sd/System/usr/trimui/scripts/$s" <<-SH
			#!/bin/sh
			echo "$s \$*" >> "$TMP/env.log"
		SH
	done
	chmod +x "$RA/ra64.trimui" "$TMP/sd/System/usr/trimui/scripts/"*.sh
}

request() {
	printf '%s\n' "$@" > "$TMP/request"
}

run() {
	SLOT_SD="$TMP/sd" SLOT_RA_HOTKEY="$TMP/ra_hotkey" sh "$SCRIPT" "$TMP/request"
}

fail() {
	echo "$1" >&2
	[ -f "$TMP/ra.log" ] && sed 's/^/  ra: /' "$TMP/ra.log" >&2
	exit 1
}

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=
run || fail "a plain game did not run"
grep -qxF "args=-v|-L|$CORE|$ROM|" "$TMP/ra.log" || fail "a plain game got the wrong arguments"
grep -qxF "home=$RA/ pwd=$RA" "$TMP/ra.log" || fail "RetroArch did not run from its own folder with HOME set"
grep -q "^lib=/usr/trimui/lib:$TMP/sd/System/lib" "$TMP/ra.log" || fail "the StockMix libraries are not first"
grep -qx "hotkey=yes" "$TMP/ra.log" || fail "RetroArch ran without its hotkeys"
[ ! -e "$TMP/ra_hotkey" ] || fail "the hotkey file slot made was left behind for Slot's MENU"
grep -qx "ra_audio_switcher.sh " "$TMP/env.log" || fail "the audio switcher did not run"
grep -qx "cpufreq.sh ondemand 2 6" "$TMP/env.log" || fail "the CPU was not set like a StockMix game"

setup
: > "$TMP/ra_hotkey"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host
run
grep -qxF "args=-v|-L|$CORE|--host|$ROM|" "$TMP/ra.log" || fail "host did not ask RetroArch to host"
[ -e "$TMP/ra_hotkey" ] || fail "a hotkey file that was already there was taken away"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=join
run
grep -qxF "args=-v|-L|$CORE|--connect|10.42.0.1|$ROM|" "$TMP/ra.log" || fail "join did not connect to the host"

setup
request mode=menu
run
grep -qxF "args=-v|" "$TMP/ra.log" || fail "the menu was not opened without a game"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host option=mgba_link=on option=mgba_link_player=0
run
grep -qxF 'opt: mgba_link = "on"' "$TMP/ra.log" || fail "the link option was not there while RetroArch ran"
grep -qxF 'opt: mgba_link_player = "0"' "$TMP/ra.log" || fail "the player was not there while RetroArch ran"
[ ! -e "$CFG/mGBA/$GAME.opt" ] || fail "the session's options were left for the next game"

setup
mkdir -p "$CFG/mGBA"
printf 'mgba_link = "off"\nmgba_skip_bios = "ON"\n' > "$CFG/mGBA/$GAME.opt"
cp "$CFG/mGBA/$GAME.opt" "$TMP/mine.opt"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=join option=mgba_link=on option=mgba_link_player=1
run
grep -qxF 'opt: mgba_skip_bios = "ON"' "$TMP/ra.log" || fail "the user's own option was dropped for the session"
grep -qxF 'opt: mgba_link = "off"' "$TMP/ra.log" && fail "the user's link setting won over the session's"
cmp -s "$CFG/mGBA/$GAME.opt" "$TMP/mine.opt" || fail "the user's options file was not given back"

setup
mkdir -p "$CFG/mGBA"
printf 'mgba_link = "on"\n' > "$CFG/mGBA/$GAME.opt"
: > "$CFG/mGBA/$GAME.opt.none"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host option=mgba_link=on option=mgba_link_player=0
run
[ ! -e "$CFG/mGBA/$GAME.opt" ] || fail "a session cut short by a power loss left its options behind for good"
[ ! -e "$CFG/mGBA/$GAME.opt.none" ] || fail "the marker of a cut session was left behind"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=
RA_EXIT=3 run && fail "a RetroArch that failed was reported as fine"
RA_EXIT=3 run || [ $? -eq 3 ] || fail "RetroArch's own exit code was not passed on"

for bad in "mode=bogus" "mode=game core=$CORE" "mode=game rom=$ROM" "mode=game core=$TMP/none.so rom=$ROM"; do
	setup
	# shellcheck disable=SC2086
	request $bad
	if run 2> /dev/null; then
		fail "[$bad] was taken as a request"
	fi
	[ ! -f "$TMP/ra.log" ] || fail "RetroArch ran for [$bad]"
done

setup
rm -f "$RA/ra64.trimui"
request mode=menu
if run 2> /dev/null; then
	fail "a card with no RetroArch reported success"
fi

echo "retroarch: all passed"
```

(No ciclo dos pedidos inválidos, `request $bad` parte em espaços de propósito, e o `$ROM` com espaços fica partido em várias linhas. Não faz mal: em todos esses casos falta um `mode` válido, um core que exista ou a ROM, que é o que o teste quer ver recusado.)

- [ ] **Step 2: Ver falhar**

Run: `sh card/test-retroarch.sh`
Expected: falha logo no primeiro caso (`a plain game did not run`), porque o script ainda não existe.

- [ ] **Step 3: Implementar `card/Apps/Slot/retroarch.sh`**

```sh
#!/bin/sh
REQUEST="$1"
SD="${SLOT_SD:-/mnt/SDCARD}"
RA_DIR="${SLOT_RA_DIR:-$SD/RetroArch}"
SCRIPTS="${SLOT_RA_SCRIPTS:-$SD/System/usr/trimui/scripts}"
HOTKEY="${SLOT_RA_HOTKEY:-/var/trimui_inputd/ra_hotkey}"
LINK_HOST="${SLOT_LINK_HOST:-10.42.0.1}"

value() {
	sed -n "s/^$1=//p" "$REQUEST" 2> /dev/null | head -n 1
}

mode="$(value mode)"
core="$(value core)"
core_name="$(value core_name)"
rom="$(value rom)"
link="$(value link)"

case "$mode" in
game)
	if [ ! -f "$core" ] || [ ! -f "$rom" ]; then
		echo "retroarch.sh: no core or no rom in $REQUEST" >&2
		exit 2
	fi
	;;
menu) ;;
*)
	echo "retroarch.sh: $REQUEST is not a request" >&2
	exit 2
	;;
esac
if [ ! -x "$RA_DIR/ra64.trimui" ]; then
	echo "retroarch.sh: no RetroArch at $RA_DIR" >&2
	exit 2
fi

export PATH="$SCRIPTS:$SD/System/bin:$PATH:$RA_DIR"
export LD_LIBRARY_PATH="/usr/trimui/lib:$SD/System/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

config_dir() {
	dir="$(sed -n 's/^rgui_config_directory = "\(.*\)"$/\1/p' "$RA_DIR/.retroarch/retroarch.cfg" 2> /dev/null | head -n 1)"
	case "$dir" in
	"" | default) echo "$RA_DIR/.retroarch/config" ;;
	"~/"*) echo "$RA_DIR/${dir#\~/}" ;;
	":/"*) echo "$RA_DIR/${dir#:/}" ;;
	"./"*) echo "$RA_DIR/${dir#./}" ;;
	/*) echo "$dir" ;;
	*) echo "$RA_DIR/$dir" ;;
	esac
}

opt=""
if [ "$mode" = game ] && [ -n "$link" ] && grep -q '^option=' "$REQUEST"; then
	game="$(basename "$rom")"
	opt_dir="$(config_dir)/$core_name"
	opt="$opt_dir/${game%.*}.opt"
	mkdir -p "$opt_dir"
	if [ ! -f "$opt.slot" ] && [ ! -f "$opt.none" ]; then
		if [ -f "$opt" ]; then
			cp -f "$opt" "$opt.slot"
		else
			: > "$opt.none"
		fi
	fi
	if [ -f "$opt.slot" ]; then
		cp -f "$opt.slot" "$opt.new"
	elif [ -f "$opt_dir/$core_name.opt" ]; then
		cp -f "$opt_dir/$core_name.opt" "$opt.new"
	else
		: > "$opt.new"
	fi
	sed -n 's/^option=//p' "$REQUEST" | while IFS='=' read -r key val; do
		grep -v "^$key = " "$opt.new" > "$opt.key"
		mv -f "$opt.key" "$opt.new"
		printf '%s = "%s"\n' "$key" "$val" >> "$opt.new"
	done
	mv -f "$opt.new" "$opt"
fi

hotkey_made=""
if [ ! -e "$HOTKEY" ] && touch "$HOTKEY" 2> /dev/null; then
	hotkey_made=1
fi
command -v ra_audio_switcher.sh > /dev/null 2>&1 && ra_audio_switcher.sh
command -v cpufreq.sh > /dev/null 2>&1 && cpufreq.sh ondemand 2 6

set --
case "$link" in
host) set -- --host ;;
join) set -- --connect "$LINK_HOST" ;;
esac
cd "$RA_DIR" || exit 2
if [ "$mode" = game ]; then
	HOME="$RA_DIR/" ./ra64.trimui -v -L "$core" "$@" "$rom"
else
	HOME="$RA_DIR/" ./ra64.trimui -v
fi
rc=$?

if [ -n "$opt" ]; then
	if [ -f "$opt.slot" ]; then
		mv -f "$opt.slot" "$opt"
	else
		rm -f "$opt"
	fi
	rm -f "$opt.none"
fi
[ -n "$hotkey_made" ] && rm -f "$HOTKEY"
exit "$rc"
```

Junta `- sh card/test-retroarch.sh` à lista `test:card` do `taskfile.yml`, depois de `- sh card/test-suspend.sh`.

- [ ] **Step 4: Ver passar**

Run: `sh card/test-retroarch.sh && shellcheck -s sh card/Apps/Slot/retroarch.sh card/test-retroarch.sh` (se o `shellcheck` estiver instalado no WSL; se não, só o teste)
Expected: `retroarch: all passed`, e o shellcheck sem avisos.

- [ ] **Step 5: Commit**

```bash
git add card/Apps/Slot/retroarch.sh card/test-retroarch.sh taskfile.yml
git commit -m "Add retroarch.sh: run a request the way StockMix's launchers run RetroArch"
```

---

### Task 9: `launch.sh` em ciclo

**Files:**
- Modify: `card/Apps/Slot/launch.sh`
- Create: `card/test-launch-retroarch.sh`
- Modify: `taskfile.yml` (`test:card`: `- sh card/test-launch-retroarch.sh`)
- Test: `card/test-launch.sh` (o existente tem de continuar a passar sem mudanças)

**Interfaces:**
- Consumes: `Slot/Config/retroarch.request` (Tarefa 6), `retroarch.sh <pedido>` (Tarefa 8), `slotlink.sh link down` (existente).
- Produces: variável `SLOT_PROC` (por omissão `/proc`) para os testes.

- [ ] **Step 1: Escrever o teste**

Cria `card/test-launch-retroarch.sh`:

```sh
#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

REQ="$TMP/sd/Slot/Config/retroarch.request"

setup() {
	rm -rf "$TMP/sd" "$TMP/app" "$TMP/bin" "$TMP/proc" "$TMP"/*.log "$TMP/runs" "$TMP/keymon.up"
	mkdir -p "$TMP/sd/Slot/Config" "$TMP/app" "$TMP/bin" "$TMP/proc/123" "$TMP/keymon-home" "$TMP/keymon-bin"
	cp "$HERE/Apps/Slot/launch.sh" "$TMP/app/launch.sh"
	echo core > "$TMP/core_pattern"

	cat > "$TMP/keymon-bin/keymon" <<-SH
		#!/bin/sh
		echo "keymon cwd=\$(pwd) lib=\$LD_LIBRARY_PATH" >> "$TMP/events.log"
		: > "$TMP/keymon.up"
	SH
	ln -s "$TMP/keymon-bin/keymon" "$TMP/proc/123/exe"
	ln -s "$TMP/keymon-home" "$TMP/proc/123/cwd"
	printf 'HOME=/root\000LD_LIBRARY_PATH=/keymon/lib\000' > "$TMP/proc/123/environ"

	cat > "$TMP/bin/pidof" <<-SH
		#!/bin/sh
		[ "\$1" = keymon ] && echo 123
	SH
	cat > "$TMP/bin/killall" <<-SH
		#!/bin/sh
		echo "killall \$*" >> "$TMP/events.log"
		rm -f "$TMP/keymon.up"
	SH

	cat > "$TMP/app/retroarch.sh" <<-SH
		#!/bin/sh
		i=0
		while [ ! -f "$TMP/keymon.up" ] && [ "\$i" -lt 30 ]; do sleep 0.1; i=\$((i + 1)); done
		[ -f "$TMP/keymon.up" ] && k=up || k=down
		echo "retroarch mode=\$(sed -n 's/^mode=//p' "\$1") keymon=\$k" >> "$TMP/events.log"
		exit \${RA_EXIT:-0}
	SH
	cat > "$TMP/app/slotlink.sh" <<-SH
		#!/bin/sh
		echo "slotlink \$*" >> "$TMP/events.log"
	SH
	chmod +x "$TMP/keymon-bin/keymon" "$TMP/bin/pidof" "$TMP/bin/killall" "$TMP/app/retroarch.sh" "$TMP/app/slotlink.sh"
}

# The fake slot: run N writes line N of $TMP/plan as the request (or nothing for "-").
slot_from_plan() {
	cat > "$TMP/app/slot" <<-SH
		#!/bin/sh
		n=\$((\$(cat "$TMP/runs" 2>/dev/null || echo 0) + 1))
		echo "\$n" > "$TMP/runs"
		[ -f "$REQ" ] && echo "slot \$n found a request" >> "$TMP/events.log"
		echo "slot \$n" >> "$TMP/events.log"
		line="\$(sed -n "\${n}p" "$TMP/plan")"
		[ -n "\$line" ] && [ "\$line" != "-" ] && printf '%b' "\$line" > "$REQ"
		exit 0
	SH
	chmod +x "$TMP/app/slot"
}

run() {
	PATH="$TMP/bin:$PATH" SLOT_SD="$TMP/sd" SLOT_CORE_PATTERN="$TMP/core_pattern" \
		SLOT_PROC="$TMP/proc" sh "$TMP/app/launch.sh"
}

expect() {
	printf '%s\n' "$@" > "$TMP/want.log"
	if ! cmp -s "$TMP/want.log" "$TMP/events.log"; then
		echo "$WHAT" >&2
		diff "$TMP/want.log" "$TMP/events.log" >&2 || true
		exit 1
	fi
}

setup
printf 'mode=game\\ncore=/c.so\\ncore_name=mGBA\\nrom=/r.gba\\nlink=host\\n\n-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a linked game did not go slot -> keymon -> RetroArch -> keymon off -> link down -> slot"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=game keymon=up" \
	"killall keymon" \
	"slotlink link down" \
	"slot 2"
[ ! -e "$REQ" ] && [ ! -e "$REQ.run" ] || { echo "the request was left on the card" >&2; exit 1; }
[ "$(cat "$TMP/core_pattern")" = core ] || { echo "the core pattern was not given back" >&2; exit 1; }

setup
printf 'mode=menu\\n\n-\n' > "$TMP/plan"
slot_from_plan
RA_EXIT=139 run
WHAT="a RetroArch that crashed did not come back to slot, or a menu without a link took the link down"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 2"

setup
printf 'mode=menu\\n' > "$REQ"
printf -- '-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a request left from before this launch opened RetroArch on its own"
expect \
	"killall keymon" \
	"slot 1"

setup
printf 'mode=menu\\n\nmode=menu\\n\n-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a second trip to RetroArch did not bring keymon back the same way"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 2" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 3"

echo "launch-retroarch: all passed"
```

Junta `- sh card/test-launch-retroarch.sh` à lista `test:card` do `taskfile.yml`, depois de `- sh card/test-launch.sh`.

- [ ] **Step 2: Ver falhar**

Run: `sh card/test-launch-retroarch.sh`
Expected: falha o primeiro caso. O `diff` mostra só `killall keymon` e `slot 1`, porque o `launch.sh` atual não tem ciclo.

- [ ] **Step 3: Implementar `card/Apps/Slot/launch.sh`**

```sh
#!/bin/sh
APP="$(cd "$(dirname "$0")" && pwd)"
SD="${SLOT_SD:-/mnt/SDCARD}"
DATA="$SD/Slot"
REQUEST="$DATA/Config/retroarch.request"
CORE_PATTERN="${SLOT_CORE_PATTERN:-/proc/sys/kernel/core_pattern}"
STOP="${SLOT_STOP-keymon}"
PROC="${SLOT_PROC:-/proc}"
TAB="$(printf '\t')"
mkdir -p "$DATA/Config"
[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
: > "$DATA/slot.log"
rm -f "$REQUEST" "$REQUEST.run"
OLD_CORE="$(cat "$CORE_PATTERN" 2>/dev/null)"
echo "|/bin/sh $APP/coresave.sh %p %s %e %t" > "$CORE_PATTERN" 2>/dev/null
export SLOT_SD="$SD" SLOT_APP="$APP"

STOPPED=""
for p in $STOP; do
	for pid in $(pidof "$p" 2>/dev/null); do
		exe="$(readlink "$PROC/$pid/exe" 2>/dev/null)" || continue
		cwd="$(readlink "$PROC/$pid/cwd" 2>/dev/null)"
		lib="$(tr '\000' '\n' < "$PROC/$pid/environ" 2>/dev/null | sed -n 's/^LD_LIBRARY_PATH=//p')"
		STOPPED="$STOPPED$exe$TAB${cwd:-/}$TAB$lib
"
		break
	done
done

stop_daemons() {
	for p in $STOP; do killall "$p" 2>/dev/null; done
}

start_daemons() {
	printf '%s' "$STOPPED" | while IFS="$TAB" read -r exe cwd lib; do
		[ -n "$exe" ] || continue
		(cd "$cwd" && LD_LIBRARY_PATH="$lib" "$exe" > /dev/null 2>&1 &)
	done
}

stop_daemons
cd "$APP" || exit 1
while :; do
	"$APP/slot" >> "$DATA/slot.log" 2>&1
	[ -f "$REQUEST" ] || break
	mv -f "$REQUEST" "$REQUEST.run"
	link="$(sed -n 's/^link=//p' "$REQUEST.run" | head -n 1)"
	start_daemons
	sh "$APP/retroarch.sh" "$REQUEST.run" > "$DATA/retroarch.log" 2>&1
	rm -f "$REQUEST.run"
	stop_daemons
	[ -n "$link" ] && sh "$APP/slotlink.sh" link down >> "$DATA/slot.log" 2>&1
	cd "$APP" || exit 1
done
[ -n "$OLD_CORE" ] && echo "$OLD_CORE" > "$CORE_PATTERN" 2>/dev/null
exit 0
```

Se a Tarefa 1 viu que o `keymon` corre com argumentos, guarda também o `cmdline` (sem o `argv[0]`) e passa-o no `start_daemons` (ruling).

- [ ] **Step 4: Ver passar, incluindo o teste antigo**

Run: `sh card/test-launch-retroarch.sh && sh card/test-launch.sh && sh card/test-boot.sh`
Expected: `launch-retroarch: all passed`, `launch: all passed` e o teste de arranque também verde.

- [ ] **Step 5: Commit**

```bash
git add card/Apps/Slot/launch.sh card/test-launch-retroarch.sh taskfile.yml
git commit -m "Loop launch.sh through RetroArch: keymon back while it runs, link down after, then slot again"
```

---

### Task 10: No Brick — gate, instalar e checklist

**Files:**
- Modify: `docs/brick-hardware.md` (§16, resultados da checklist)

- [ ] **Step 1: Gate completa**

Run: o `gate.sh` do scratchpad (fmt, clippy host e device, testes e cartão).
Expected: `gate PASS`, com mais testes do que os 1380 do fim da Tarefa 22 e 0 falhas além das 9 conhecidas.

- [ ] **Step 2: Build e instalação**

Run: `task dist:device` e depois a linha de `tar | ssh brick` da Tarefa 1, Step 2, com `ls -la /mnt/SDCARD/Apps/Slot/retroarch.sh` no fim.
Expected: `retroarch.sh` listado e executável.

- [ ] **Step 3: Checklist com o utilizador**

Pede ao utilizador para abrir o Slot em Apps, ligar Use RetroArch no menu (MENU) e fazer:

| # | Teste | Esperado |
|---|---|---|
| 1 | Meter um cartucho GBA com o mGBA escolhido | Abre no RetroArch, com som e botões |
| 2 | O mesmo com o gpSP (START no cartucho) | Abre no RetroArch com o gpSP |
| 3 | Um cartucho GB | Abre no RetroArch com o mGBA |
| 4 | Gravar no jogo e sair pelo menu do RetroArch | Volta à prateleira com o mesmo cartucho em frente; o save aparece em `RetroArch/.retroarch/saves` e no cloud sync |
| 5 | Vol±, brilho e power (adormecer) dentro do RetroArch | Como num jogo do StockMix |
| 6 | Na prateleira depois de voltar: MENU, Vol±, power | O menu do Slot abre; não aparece o pop-up de volume do sistema |
| 7 | RetroArch Settings | Abre o menu do RetroArch e volta ao Slot |
| 8 | Desligar Use RetroArch e meter um cartucho | O Slot joga como antes |
| 9 | Com dois Bricks: Host num, Join no outro (SELECT no cartucho) | Mesmo jogo ligado pelo netplay do RetroArch |

Depois de cada falha:
```bash
ssh brick 'tail -40 /mnt/SDCARD/Slot/slot.log; tail -60 /mnt/SDCARD/Slot/retroarch.log'
```
Usa systematic-debugging antes de qualquer correção. Cada correção é feita com TDD num commit próprio.

- [ ] **Step 4: Registar e fazer commit**

Acrescenta à §16 de `docs/brick-hardware.md` o resultado de cada linha da checklist. Linhas não testadas (por exemplo a 9 sem segundo Brick) ficam como "não testado" e dizem porquê.

```bash
git add docs/brick-hardware.md
git commit -m "Record the RetroArch mode checklist on the Brick"
```
