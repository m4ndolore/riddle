//! Path B "startup ask": the launch script snapshots xochitl's screen (raw
//! gray8 read out of its process memory) before our window covers it, and
//! points RIDDLE_ASK_RAW at the dump. At startup we turn that dump into the
//! oracle PNG ourselves — the tablet has no image tools — so the answer to
//! what you wrote in the stock notes app writes itself onto the blank page.

use crate::fb::{SCREEN_H, SCREEN_W};

/// The dump is panel-native landscape: SCREEN_H wide, SCREEN_W tall.
const RAW_W: usize = SCREEN_H;
const RAW_H: usize = SCREEN_W;

/// Convert the raw capture at `raw_path` into an oracle-ready grayscale PNG
/// at `png_path`. Consumes the dump (renames it to *.asked) so a relaunch
/// doesn't re-ask a stale page. Returns None — with a log line — on any
/// mismatch: startup must never fail because a capture went wrong.
pub fn prepare(raw_path: &str, png_path: &str) -> Option<()> {
    let raw = match std::fs::read(raw_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("riddle: ask capture {raw_path} unreadable ({e}); skipped");
            return None;
        }
    };
    let _ = std::fs::rename(raw_path, format!("{raw_path}.asked"));
    if raw.len() != RAW_W * RAW_H {
        eprintln!(
            "riddle: ask capture is {} bytes, want {} ({}x{} gray8); skipped",
            raw.len(),
            RAW_W * RAW_H,
            RAW_W,
            RAW_H
        );
        return None;
    }

    // The panel is mounted rotated; which way maps memory to the upright page
    // is per-device lore, so keep it overridable until verified on hardware.
    let rot = std::env::var("RIDDLE_ASK_ROT").unwrap_or_else(|_| "ccw".into());
    let (pw, ph) = if rot == "none" { (RAW_W, RAW_H) } else { (RAW_H, RAW_W) };
    let px = |x: usize, y: usize| -> u8 {
        match rot.as_str() {
            "none" => raw[y * RAW_W + x],
            "cw" => raw[(RAW_H - 1 - x) * RAW_W + y],
            _ => raw[x * RAW_W + (RAW_W - 1 - y)], // ccw
        }
    };

    // Box-downscale so the long side stays ≤ 800px (at least 2x) — same
    // budget as ink::to_png: the model reads handwriting fine at that scale
    // and image pixels are the dominant vision-token / latency cost.
    let f = pw.max(ph).div_ceil(800).max(2);
    let (w, h) = (pw / f, ph / f);
    let mut gray = vec![0u8; w * h];
    for oy in 0..h {
        for ox in 0..w {
            let mut acc = 0u32;
            for sy in 0..f {
                for sx in 0..f {
                    acc += px(ox * f + sx, oy * f + sy) as u32;
                }
            }
            gray[oy * w + ox] = (acc / (f * f) as u32) as u8;
        }
    }

    let file = match std::fs::File::create(png_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("riddle: ask png {png_path} unwritable ({e}); skipped");
            return None;
        }
    };
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    let ok = enc
        .write_header()
        .and_then(|mut wr| wr.write_image_data(&gray))
        .map_err(|e| eprintln!("riddle: ask png encode failed: {e}"));
    ok.ok().map(|_| ())
}
