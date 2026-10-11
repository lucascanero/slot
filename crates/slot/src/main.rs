#[cfg(not(feature = "host"))]
mod device_app;
#[cfg(feature = "host")]
mod host_app;

fn main() {
    let mut args = std::env::args().skip(1);
    let flag = args.next();
    if let Some(flag @ ("--init-root" | "--init-sd")) = flag.as_deref() {
        let Some(dir) = args.next() else {
            eprintln!("slot: {flag} needs a directory");
            std::process::exit(2);
        };
        let dir = std::path::Path::new(&dir);
        match flag {
            "--init-sd" => slot::root::ensure_layout(&slot_store::Layout::stockmix(
                dir,
                &dir.join("Apps/Slot"),
            )),
            _ => slot::root::ensure(dir),
        }
        return;
    }
    slot::crash::install();

    #[cfg(feature = "host")]
    host_app::run();

    #[cfg(not(feature = "host"))]
    device_app::run();
}
