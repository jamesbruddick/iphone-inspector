//! The tray icon, drawn in code from the same shapes as the favicon: a phone outline with a check mark.
//!
//! On macOS it is a template image - black on transparent - so the menu bar tints it for light and
//! dark mode itself. Windows and Linux have no such thing and their taskbars come in both shades, so
//! there it keeps the favicon's indigo tile, which reads on either.

use tray_icon::Icon;

/// Rounded-rectangle signed distance: negative inside, positive outside.
fn round_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (px - (x + w / 2.0)).abs() - (w / 2.0 - r);
    let qy = (py - (y + h / 2.0)).abs() - (h / 2.0 - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn segment(px: f32, py: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32)) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

/// Coverage of the phone-and-check-mark shape at a point in the favicon's 32-unit space.
fn mark(x: f32, y: f32) -> bool {
    let outline = round_rect(x, y, 10.0, 6.0, 12.0, 20.0, 2.5).abs() <= 0.9;
    let check = segment(x, y, (12.6, 16.4), (14.9, 18.8)).min(segment(x, y, (14.9, 18.8), (19.4, 13.8))) <= 1.0;
    outline || check
}

pub fn tray_icon() -> Icon {
    let template = cfg!(target_os = "macos");
    // The template is cropped to the phone so it fills the menu-bar slot; the tile keeps its margin.
    let (size, from, span) = if template { (44u32, 4.0f32, 24.0f32) } else { (64, 0.0, 32.0) };
    const SAMPLES: u32 = 4;

    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let (mut tile, mut ink) = (0.0f32, 0.0f32);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = from + (px as f32 + (sx as f32 + 0.5) / SAMPLES as f32) * span / size as f32;
                    let y = from + (py as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * span / size as f32;
                    if round_rect(x, y, 0.0, 0.0, 32.0, 32.0, 7.0) <= 0.0 {
                        tile += 1.0;
                    }
                    if mark(x, y) {
                        ink += 1.0;
                    }
                }
            }
            let n = (SAMPLES * SAMPLES) as f32;
            let (tile, ink) = (tile / n, ink / n);
            if template {
                rgba.extend_from_slice(&[0, 0, 0, (ink * 255.0).round() as u8]);
            } else {
                // White mark over the indigo tile (#4f46e5), blended by coverage.
                let blend = |c: f32| (c + (255.0 - c) * ink).round() as u8;
                rgba.extend_from_slice(&[blend(79.0), blend(70.0), blend(229.0), (tile * 255.0).round() as u8]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("icon dimensions match its pixel buffer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_where_the_favicon_puts_it() {
        // On the phone's outline, on the check mark, and in the empty screen between them.
        assert!(mark(10.0, 16.0));
        assert!(mark(14.9, 18.8));
        assert!(!mark(16.0, 9.0));
        // Outside the phone entirely.
        assert!(!mark(3.0, 3.0));
    }

    #[test]
    fn preview() {
        // `cargo test preview -- --nocapture` draws the macOS template as text.
        for y in (0..24).map(|y| 4.0 + y as f32) {
            let row: String = (0..24).map(|x| if mark(4.0 + x as f32 + 0.5, y + 0.5) { '#' } else { '.' }).collect();
            println!("{row}");
        }
        let _ = tray_icon();
    }
}
