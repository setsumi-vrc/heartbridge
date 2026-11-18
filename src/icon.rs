//! Icons drawn in code (no image files): the heart for the window and the
//! tray icon with the live BPM in pixel digits.

/// RGBA of a heart (implicit equation (x²+y²−1)³ − x²y³ ≤ 0) with 4×4 antialiasing.
pub fn heart_rgba(size: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for py in 0..size {
        for px in 0..size {
            let mut hits = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = ((px as f32 + (sx as f32 + 0.5) / 4.0) / s - 0.5) * 2.7;
                    let y = -((py as f32 + (sy as f32 + 0.5) / 4.0) / s - 0.5) * 2.7 + 0.15;
                    let a = x * x + y * y - 1.0;
                    if a * a * a - x * x * y * y * y <= 0.0 {
                        hits += 1;
                    }
                }
            }
            let i = ((py * size + px) * 4) as usize;
            out[i..i + 3].copy_from_slice(&rgb);
            out[i + 3] = (hits * 255 / 16) as u8;
        }
    }
    out
}

/// 3×5 digits, one row per byte (3 low bits).
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

/// 32×32 tray icon: red rounded square with the BPM in white.
/// Digits are 2 px per cell on even coordinates, so they stay crisp when
/// Windows scales the icon down to 16 px.
pub fn tray_bpm_rgba(bpm: u16, bg: [u8; 3]) -> Vec<u8> {
    const N: i32 = 32;
    let mut out = vec![0u8; (N * N * 4) as usize];
    let radius = 7.0f32;
    for y in 0..N {
        for x in 0..N {
            // Rounded corners with soft edges.
            let cx = (x as f32 + 0.5).clamp(radius, N as f32 - radius);
            let cy = (y as f32 + 0.5).clamp(radius, N as f32 - radius);
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let alpha = (radius + 0.5 - d).clamp(0.0, 1.0);
            let i = ((y * N + x) * 4) as usize;
            out[i..i + 3].copy_from_slice(&bg);
            out[i + 3] = (alpha * 255.0) as u8;
        }
    }

    let text = bpm.min(999).to_string();
    let n = text.len() as i32;
    let width = n * 6 + (n - 1) * 2;
    let x0 = ((N - width) / 2) & !1;
    let y0 = 10;
    for (k, ch) in text.bytes().enumerate() {
        let glyph = DIGITS[(ch - b'0') as usize];
        let gx = x0 + k as i32 * 8;
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0 {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let x = gx + col * 2 + dx;
                            let y = y0 + row as i32 * 2 + dy;
                            let i = ((y * N + x) * 4) as usize;
                            out[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heart_has_shape() {
        let px = heart_rgba(32, [255, 0, 0]);
        let alpha = |x: usize, y: usize| px[(y * 32 + x) * 4 + 3];
        assert_eq!(alpha(16, 16), 255, "the centre is filled");
        assert_eq!(alpha(0, 0), 0, "the corner is empty");
    }

    #[test]
    fn tray_digits() {
        let px = tray_bpm_rgba(128, [255, 59, 71]);
        assert_eq!(px.len(), 32 * 32 * 4);
        let white = px.chunks(4).filter(|p| p == &[255, 255, 255, 255]).count();
        assert!(white > 40, "the digits are drawn");
    }
}
