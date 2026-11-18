//! Embeds the app icon and version info into the Windows executable.
//! The icon is drawn by the same code the window uses (src/icon.rs), so
//! there is no image file to keep in sync.

#[cfg(windows)]
#[allow(dead_code)]
#[path = "src/icon.rs"]
mod icon;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=build.rs");

    // Only when building on Windows for Windows (needs rc.exe).
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_icon();
    }
}

#[cfg(windows)]
fn embed_icon() {
    use std::path::PathBuf;

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("heartbridge.ico");
    std::fs::write(&out, ico(&[16, 20, 24, 32, 40, 48, 64, 256])).expect("write icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(out.to_str().unwrap());
    res.set("ProductName", "HeartBridge");
    res.set("FileDescription", "HeartBridge");
    if let Err(e) = res.compile() {
        // A missing resource compiler shouldn't stop the build; the exe just
        // has no embedded icon.
        println!("cargo:warning=could not embed the icon: {e}");
    }
}

/// .ico file with one 32-bit BMP image per size.
#[cfg(windows)]
fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| bmp(s)).collect();

    let mut f = Vec::new();
    f.extend_from_slice(&[0, 0, 1, 0]);
    f.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&images) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        f.extend_from_slice(&[dim, dim, 0, 0]);
        f.extend_from_slice(&1u16.to_le_bytes()); // planes
        f.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        f.extend_from_slice(&(img.len() as u32).to_le_bytes());
        f.extend_from_slice(&offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        f.extend_from_slice(&img);
    }
    f
}

/// BITMAPINFOHEADER + bottom-up BGRA pixels + an empty AND mask.
#[cfg(windows)]
fn bmp(size: u32) -> Vec<u8> {
    let rgba = icon::heart_rgba(size, [0xFF, 0x3B, 0x47]);
    let mask_row = size.div_ceil(32) * 4;

    let mut b = Vec::new();
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(size as i32).to_le_bytes());
    b.extend_from_slice(&(size as i32 * 2).to_le_bytes()); // colour + mask
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]); // no compression, sizes and palette unused
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            b.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    b.extend(std::iter::repeat_n(0u8, (mask_row * size) as usize));
    b
}
