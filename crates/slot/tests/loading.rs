use slot::loading::{centre, draw, lit_at, write_frames, BG_FILE, DOT_MS, FRAMES, INK, PANEL};
use slot_gfx::{screen_rect, OUT_H, OUT_W};
use slot_ui::{opening, Draw};

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
fn each_frame_lights_its_own_dot_on_the_loading_colour() {
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
    assert_eq!(
        px(first, *w, 0, 0),
        ground(),
        "the frame's corner is not the loading colour"
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
fn the_loading_screen_covers_slot_in_the_loading_colour_and_fades_in() {
    let mut out = Vec::new();
    draw(0, 0.5, &mut out);
    let Some(Draw::Rect { x, y, w, h, colour }) = out.first().copied() else {
        panic!("no cover: {out:?}");
    };
    assert_eq!([x, y, w, h], [0.0, 0.0, OUT_W as f32, OUT_H as f32]);
    assert_eq!(colour[..3], opening()[..3]);
    assert_eq!(colour[3], 0.5);
    assert!(out.len() > 4, "no icon on the cover");
}

#[test]
fn the_lit_dot_walks_one_step_every_dot_ms() {
    assert_eq!(lit_at(0), 0);
    assert_eq!(lit_at(DOT_MS), 1);
    assert_eq!(lit_at(2 * DOT_MS), 2);
    assert_eq!(lit_at(3 * DOT_MS), 0);
}
