use std::fs;
use std::os::unix::fs::PermissionsExt;

use slot_store::{BootHook, Layout};

#[test]
fn turning_it_on_copies_the_apps_script_into_the_boot_folder() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    fs::create_dir_all(l.boot_script().parent().unwrap()).unwrap();
    fs::write(l.boot_script(), "#!/bin/sh\necho slot\n").unwrap();
    let hook = BootHook::for_layout(&l);
    assert!(!hook.installed());
    hook.set(true).unwrap();
    assert!(hook.installed());
    assert_eq!(
        fs::read_to_string(l.boot_hook()).unwrap(),
        "#!/bin/sh\necho slot\n"
    );
    let mode = fs::metadata(l.boot_hook()).unwrap().permissions().mode();
    assert_ne!(mode & 0o111, 0, "the boot script cannot be run");
    hook.set(false).unwrap();
    assert!(!hook.installed());
    hook.set(false).unwrap();
}

#[test]
fn a_missing_script_fails_and_leaves_it_off() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    let hook = BootHook::for_layout(&l);
    assert!(hook.set(true).is_err());
    assert!(!hook.installed());
}
