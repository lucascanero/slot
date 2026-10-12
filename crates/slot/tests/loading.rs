use slot::loading::{
    centre, draw, lit_at, read_screen, write_background, write_frames, BADGE, BG_FILE, DOT_MS,
    FRAMES, INK, PANEL,
};
use slot_gfx::{screen_rect, OUT_H, OUT_W};
use slot_ui::{edge, opening, Draw};

fn decode(path: &std::path::Path) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::fs::File::open(path).expect("open png"));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}", path.display());
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn px(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * w + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

fn ground() -> [u8; 3] {
    let c = opening();
    [
        (c[0] * 255.0).round() as u8,
        (c[1] * 255.0).round() as u8,
        (c[2] * 255.0).round() as u8,
    ]
}

fn top() -> u32 {
    let (_, _, _, h) = screen_rect(PANEL);
    PANEL.1 - h as u32
}

#[test]
fn the_background_is_the_docked_slot_screen_in_its_own_colour() {
    let d = tempfile::tempdir().unwrap();
    write_frames(d.path()).unwrap();
    let (w, h, rgba) = decode(&d.path().join(BG_FILE));
    assert_eq!((w, h), PANEL);
    assert_eq!(
        px(&rgba, w, 10, top() - 1),
        [0, 0, 0],
        "the strip above the docked screen is not black"
    );
    assert_eq!(
        px(&rgba, w, 10, top()),
        ground(),
        "the docked screen is not the loading colour"
    );
    assert_eq!(px(&rgba, w, w / 2, h - 1), ground());
}

#[test]
fn each_frame_lights_its_own_dot_inside_the_badge() {
    let d = tempfile::tempdir().unwrap();
    write_frames(d.path()).unwrap();
    let frames: Vec<(u32, u32, Vec<u8>)> = (1..=FRAMES)
        .map(|i| decode(&d.path().join(format!("app_loading_{i:02}.png"))))
        .collect();
    let (w, h, first) = &frames[0];
    assert!(
        *w < PANEL.0 / 4 && *h < PANEL.1 / 4,
        "a frame is not a small icon: {w}x{h}"
    );
    assert!(
        first.chunks(4).any(|p| p[..3] == ground()),
        "the badge has no loading colour inside"
    );
    assert!(
        first.chunks(4).any(|p| p[..3] == INK),
        "the icon is not drawn in the menu ink"
    );
    for (i, (fw, fh, rgba)) in frames.iter().enumerate() {
        assert_eq!((fw, fh), (w, h), "frame {} is a different size", i + 1);
        for (j, (_, _, other)) in frames.iter().enumerate() {
            assert!(
                i == j || rgba != other,
                "frames {} and {} are the same",
                i + 1,
                j + 1
            );
        }
    }
}

#[test]
fn the_icon_slot_draws_lands_where_retroarch_centres_its_frames() {
    let (cx, cy) = centre();
    let (_, _, w, h) = screen_rect(PANEL);
    let x = cx * w as f32 / OUT_W as f32;
    let y = top() as f32 + cy * h as f32 / OUT_H as f32;
    assert!((x - PANEL.0 as f32 / 2.0).abs() < 1.0, "x {x}");
    assert!((y - PANEL.1 as f32 / 2.0).abs() < 1.0, "y {y}");
}

#[test]
fn the_loading_badge_sits_on_the_centre_over_whatever_slot_shows() {
    let mut out = Vec::new();
    draw(0, 0.5, &mut out);
    let (cx, cy) = centre();
    let Some(Draw::Rect { x, y, w, h, colour }) = out.first().copied() else {
        panic!("no badge: {out:?}");
    };
    assert_eq!(
        [x, y, w, h],
        [cx - BADGE.0, cy - BADGE.1, 2.0 * BADGE.0, 2.0 * BADGE.1]
    );
    assert_eq!(colour[..3], edge()[..3], "the badge has no edge");
    for d in &out {
        let Draw::Rect { w, h, colour, .. } = *d else {
            panic!("the badge is made of rects: {d:?}");
        };
        assert!(
            w < OUT_W as f32 / 4.0 && h < OUT_H as f32 / 4.0,
            "the badge covers the cart"
        );
        assert_eq!(colour[3], 0.5, "part of the badge did not fade with it");
    }
}

#[test]
fn a_frame_is_the_badge_slot_draws_at_the_panels_scale() {
    let d = tempfile::tempdir().unwrap();
    write_frames(d.path()).unwrap();
    let (w, h, rgba) = decode(&d.path().join("app_loading_01.png"));
    let (_, _, sw, sh) = screen_rect(PANEL);
    let want_w = 2.0 * BADGE.0 * sw as f32 / OUT_W as f32;
    let want_h = 2.0 * BADGE.1 * sh as f32 / OUT_H as f32;
    assert!((w as f32 - want_w).abs() <= 2.0, "width {w} vs {want_w}");
    assert!((h as f32 - want_h).abs() <= 2.0, "height {h} vs {want_h}");
    let e = edge();
    let edge_rgb = [
        (e[0] * 255.0).round() as u8,
        (e[1] * 255.0).round() as u8,
        (e[2] * 255.0).round() as u8,
    ];
    assert_eq!(
        px(&rgba, w, 0, 0),
        edge_rgb,
        "the frame's corner is not the badge edge"
    );
}

fn fake_fb(dir: &std::path::Path, page: u32) -> (std::path::PathBuf, std::path::PathBuf) {
    let sys = dir.join("sys");
    std::fs::create_dir_all(&sys).unwrap();
    std::fs::write(sys.join("stride"), "4096\n").unwrap();
    std::fs::write(sys.join("bits_per_pixel"), "32\n").unwrap();
    std::fs::write(sys.join("pan"), format!("0,{}\n", page * PANEL.1)).unwrap();
    let mut fb = vec![0u8; (4096 * PANEL.1 * 2) as usize];
    let at = ((page * PANEL.1 + 5) * 4096 + 7 * 4) as usize;
    fb[at..at + 4].copy_from_slice(&[0x30, 0x20, 0x10, 0x00]);
    let path = dir.join("fb0");
    std::fs::write(&path, fb).unwrap();
    (path, sys)
}

#[test]
fn the_screen_is_read_from_the_page_on_show_as_rgba() {
    let d = tempfile::tempdir().unwrap();
    let (fb, sys) = fake_fb(d.path(), 1);
    let (w, h, rgba) = read_screen(&fb, &sys).unwrap();
    assert_eq!((w, h), PANEL);
    let at = ((5 * w + 7) * 4) as usize;
    assert_eq!(
        &rgba[at..at + 4],
        &[0x10, 0x20, 0x30, 0xff],
        "BGRA was not turned into RGBA"
    );
    assert_eq!(&rgba[0..4], &[0, 0, 0, 0xff]);
}

#[test]
fn a_screen_that_cannot_be_read_is_an_error_not_a_picture() {
    let d = tempfile::tempdir().unwrap();
    let (fb, sys) = fake_fb(d.path(), 0);
    std::fs::write(sys.join("bits_per_pixel"), "16\n").unwrap();
    assert!(
        read_screen(&fb, &sys).is_err(),
        "a 16-bit screen was read as 32-bit"
    );
    assert!(read_screen(&d.path().join("none"), &sys).is_err());
}

#[test]
fn the_screen_slot_left_becomes_retroarchs_loading_background() {
    let d = tempfile::tempdir().unwrap();
    write_frames(d.path()).unwrap();
    let mut rgba = vec![0u8; (PANEL.0 * PANEL.1 * 4) as usize];
    rgba[..4].copy_from_slice(&[1, 2, 3, 0xff]);
    write_background(d.path(), PANEL.0, PANEL.1, &rgba).unwrap();
    let (w, h, back) = decode(&d.path().join(BG_FILE));
    assert_eq!((w, h), PANEL);
    assert_eq!(px(&back, w, 0, 0), [1, 2, 3]);
}

#[test]
fn the_lit_dot_walks_one_step_every_dot_ms() {
    assert_eq!(lit_at(0), 0);
    assert_eq!(lit_at(DOT_MS), 1);
    assert_eq!(lit_at(2 * DOT_MS), 2);
    assert_eq!(lit_at(3 * DOT_MS), 0);
}
