use std::io::BufWriter;
use std::path::Path;

use slot_gfx::{screen_rect, OUT_H, OUT_W};
use slot_ui::{opening, Draw, Millis};

pub const PANEL: (u32, u32) = (1024, 768);
pub const FRAMES: usize = 3;
pub const DOT_MS: Millis = 220;
pub const LOADING_DIR: &str = "Config/loading";
pub const BG_FILE: &str = "app_loading_bg.png";

pub const INK: [u8; 3] = [0xf6, 0xf4, 0xef];
const DIM: [u8; 3] = [0x5c, 0x5c, 0x66];

const HALF_W: f32 = 17.0;
const HALF_H: f32 = 29.0;
const PAD: f32 = 4.0;

#[derive(Copy, Clone)]
enum Paint {
    Ink,
    Dim,
    Ground,
}

struct Part {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    paint: Paint,
}

fn parts(lit: usize) -> Vec<Part> {
    let part = |x, y, w, h, paint| Part { x, y, w, h, paint };
    let mut v = vec![
        part(-17.0, -26.0, 34.0, 40.0, Paint::Ink),
        part(-14.0, -23.0, 28.0, 34.0, Paint::Ground),
        part(-10.0, -19.0, 20.0, 14.0, Paint::Ink),
    ];
    for i in 0..FRAMES {
        let paint = if i == lit { Paint::Ink } else { Paint::Dim };
        v.push(part(-15.5 + 12.0 * i as f32, 22.0, 7.0, 7.0, paint));
    }
    v
}

fn ground() -> [u8; 3] {
    let c = opening();
    [
        (c[0] * 255.0).round() as u8,
        (c[1] * 255.0).round() as u8,
        (c[2] * 255.0).round() as u8,
    ]
}

fn rgb(paint: Paint) -> [u8; 3] {
    match paint {
        Paint::Ink => INK,
        Paint::Dim => DIM,
        Paint::Ground => ground(),
    }
}

fn docked() -> (u32, f32, f32) {
    let (_, _, w, h) = screen_rect(PANEL);
    let top = PANEL.1 - h as u32;
    (top, w as f32 / OUT_W as f32, h as f32 / OUT_H as f32)
}

pub fn centre() -> (f32, f32) {
    let (top, _, sy) = docked();
    (OUT_W as f32 / 2.0, (PANEL.1 as f32 / 2.0 - top as f32) / sy)
}

pub fn lit_at(now: Millis) -> usize {
    ((now / DOT_MS) % FRAMES as Millis) as usize
}

pub fn draw(lit: usize, alpha: f32, out: &mut Vec<Draw>) {
    let g = opening();
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: OUT_W as f32,
        h: OUT_H as f32,
        colour: [g[0], g[1], g[2], alpha],
    });
    let (cx, cy) = centre();
    for p in parts(lit) {
        let c = rgb(p.paint);
        out.push(Draw::Rect {
            x: cx + p.x,
            y: cy + p.y,
            w: p.w,
            h: p.h,
            colour: [
                f32::from(c[0]) / 255.0,
                f32::from(c[1]) / 255.0,
                f32::from(c[2]) / 255.0,
                alpha,
            ],
        });
    }
}

fn fill(rgba: &mut [u8], w: u32, area: [u32; 4], c: [u8; 3]) {
    let [x0, y0, x1, y1] = area;
    for y in y0..y1 {
        for x in x0..x1 {
            let at = ((y * w + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[c[0], c[1], c[2], 0xff]);
        }
    }
}

fn frame(lit: usize) -> (u32, u32, Vec<u8>) {
    let (_, sx, sy) = docked();
    let w = (2.0 * (HALF_W + PAD) * sx).round() as u32 & !1;
    let h = (2.0 * (HALF_H + PAD) * sy).round() as u32 & !1;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    fill(&mut rgba, w, [0, 0, w, h], ground());
    let (ox, oy) = (w as f32 / 2.0, h as f32 / 2.0);
    for p in parts(lit) {
        let edge = |v: f32, max: u32| (v.round().max(0.0) as u32).min(max);
        let area = [
            edge(ox + p.x * sx, w),
            edge(oy + p.y * sy, h),
            edge(ox + (p.x + p.w) * sx, w),
            edge(oy + (p.y + p.h) * sy, h),
        ];
        fill(&mut rgba, w, area, rgb(p.paint));
    }
    (w, h, rgba)
}

fn background() -> Vec<u8> {
    let (top, _, _) = docked();
    let (w, h) = PANEL;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    fill(&mut rgba, w, [0, 0, w, top], [0, 0, 0]);
    fill(&mut rgba, w, [0, top, w, h], ground());
    rgba
}

fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(rgba).map_err(std::io::Error::other)
}

pub fn write_frames(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    write_png(&dir.join(BG_FILE), PANEL.0, PANEL.1, &background())?;
    for lit in 0..FRAMES {
        let (w, h, rgba) = frame(lit);
        write_png(
            &dir.join(format!("app_loading_{:02}.png", lit + 1)),
            w,
            h,
            &rgba,
        )?;
    }
    Ok(())
}
