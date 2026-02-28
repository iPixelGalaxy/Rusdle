fn main() {
    // Only embed Windows resources when building FOR Windows
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let icon_path = format!("{}/rusdle.ico", out_dir);
    write_ico(&icon_path);

    // winres is only present as a build dep on cfg(windows), so gate the call
    #[cfg(windows)]
    {
        if let Err(e) = winres::WindowsResource::new()
            .set_icon(&icon_path)
            .compile()
        {
            eprintln!("cargo:warning=Could not embed Windows icon: {e}");
        }
    }
}

// ── ICO generation ────────────────────────────────────────────────────────────

/// Draw a single ICO file containing 16×16 and 32×32 BMP images.
/// Design: Rusdle-green (#538D4E) background with a white block "R".
fn write_ico(path: &str) {
    let img32 = make_image(32);
    let img16 = make_image(16);

    let bmp32 = bmp_dib(&img32, 32);
    let bmp16 = bmp_dib(&img16, 16);

    let ico = pack_ico(&[(&bmp32, 32u8), (&bmp16, 16u8)]);
    std::fs::write(path, &ico).expect("Failed to write icon");
}

/// Generate BGRA pixel buffer for a `size`×`size` icon.
fn make_image(size: u32) -> Vec<[u8; 4]> {
    // Rusdle green (#538D4E) in BGRA
    let green: [u8; 4] = [0x4E, 0x8D, 0x53, 0xFF];
    let white: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

    let mut px = vec![green; (size * size) as usize];

    // Slightly darker corners for a subtle rounded feel (2px inset)
    let dark: [u8; 4] = [0x38, 0x6B, 0x3C, 0xFF];
    let s = size as usize;
    let corners = [(0, 0), (1, 0), (0, 1), (s - 1, 0), (s - 2, 0), (s - 1, 1),
                   (0, s - 1), (1, s - 1), (0, s - 2), (s - 1, s - 1), (s - 2, s - 1),
                   (s - 1, s - 2)];
    for (x, y) in corners { px[y * s + x] = dark; }

    // "R" bitmap: 6 columns × 7 rows (thin stroke)
    //  # # # # . .
    //  # . . . # .
    //  # . . . # .
    //  # # # # . .
    //  # . # . . .
    //  # . . # . .
    //  # . . . # .
    const R_COLS: usize = 6;
    const R_ROWS: usize = 7;
    let r_bits: [[u8; R_COLS]; R_ROWS] = [
        [1, 1, 1, 1, 0, 0],
        [1, 0, 0, 0, 1, 0],
        [1, 0, 0, 0, 1, 0],
        [1, 1, 1, 1, 0, 0],
        [1, 0, 1, 0, 0, 0],
        [1, 0, 0, 1, 0, 0],
        [1, 0, 0, 0, 1, 0],
    ];

    // Scale so the R fills roughly 55% of the icon height
    let scale = ((size as usize * 55 / 100) / R_ROWS).max(1);
    let rw = R_COLS * scale;
    let rh = R_ROWS * scale;
    let x0 = (size as usize).saturating_sub(rw) / 2;
    let y0 = (size as usize).saturating_sub(rh) / 2;

    for (row, bits) in r_bits.iter().enumerate() {
        for (col, &bit) in bits.iter().enumerate() {
            if bit == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = x0 + col * scale + dx;
                        let y = y0 + row * scale + dy;
                        if x < s && y < s {
                            px[y * s + x] = white;
                        }
                    }
                }
            }
        }
    }

    px
}

/// Encode pixel buffer as a BMP DIB (no file header) for use inside an ICO.
fn bmp_dib(pixels: &[[u8; 4]], size: u32) -> Vec<u8> {
    let mut d = Vec::new();
    // BITMAPINFOHEADER (40 bytes)
    d.extend_from_slice(&40u32.to_le_bytes());        // biSize
    d.extend_from_slice(&size.to_le_bytes());          // biWidth
    d.extend_from_slice(&(size * 2).to_le_bytes());    // biHeight (×2 for mask)
    d.extend_from_slice(&1u16.to_le_bytes());          // biPlanes
    d.extend_from_slice(&32u16.to_le_bytes());         // biBitCount (32-bit BGRA)
    d.extend_from_slice(&0u32.to_le_bytes());          // biCompression
    d.extend_from_slice(&0u32.to_le_bytes());          // biSizeImage
    d.extend_from_slice(&0u32.to_le_bytes());          // biXPelsPerMeter
    d.extend_from_slice(&0u32.to_le_bytes());          // biYPelsPerMeter
    d.extend_from_slice(&0u32.to_le_bytes());          // biClrUsed
    d.extend_from_slice(&0u32.to_le_bytes());          // biClrImportant

    // XOR (colour) data — bottom-to-top row order
    for row in (0..size as usize).rev() {
        for col in 0..size as usize {
            d.extend_from_slice(&pixels[row * size as usize + col]);
        }
    }

    // AND mask — all zeros = fully opaque, padded to DWORD boundary per row
    let mask_stride = ((size + 31) / 32 * 4) as usize;
    for _ in 0..size {
        for _ in 0..mask_stride {
            d.push(0);
        }
    }

    d
}

/// Pack one or more BMP DIBs into a valid ICO file.
fn pack_ico(images: &[(&[u8], u8)]) -> Vec<u8> {
    let count = images.len() as u16;
    let dir_offset: u32 = 6 + count as u32 * 16;

    let mut ico = Vec::new();
    // ICONDIR header
    ico.extend_from_slice(&[0, 0]);         // reserved
    ico.extend_from_slice(&[1, 0]);         // type = ICO
    ico.extend_from_slice(&count.to_le_bytes());

    // Directory entries
    let mut offset = dir_offset;
    for (data, sz) in images {
        ico.push(*sz);                                    // bWidth
        ico.push(*sz);                                    // bHeight
        ico.push(0);                                      // bColorCount
        ico.push(0);                                      // reserved
        ico.extend_from_slice(&1u16.to_le_bytes());       // wPlanes
        ico.extend_from_slice(&32u16.to_le_bytes());      // wBitCount
        ico.extend_from_slice(&(data.len() as u32).to_le_bytes()); // dwBytesInRes
        ico.extend_from_slice(&offset.to_le_bytes());     // dwImageOffset
        offset += data.len() as u32;
    }

    // Image data
    for (data, _) in images {
        ico.extend_from_slice(data);
    }

    ico
}
