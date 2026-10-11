use std::path::Path;

use slot_store::Layout;

#[test]
fn the_installed_layout_answers_for_its_own_root_only() {
    let l = Layout::stockmix(Path::new("/sd"), Path::new("/sd/Apps/Slot"));
    assert!(l.clone().install());
    assert_eq!(Layout::for_root(Path::new("/sd/Slot")), l);
    assert_eq!(Layout::installed(), Some(&l));
    assert_eq!(
        Layout::for_root(Path::new("/elsewhere")),
        Layout::flat(Path::new("/elsewhere"))
    );
    assert!(
        !Layout::flat(Path::new("/x")).install(),
        "a second install is refused"
    );
}
