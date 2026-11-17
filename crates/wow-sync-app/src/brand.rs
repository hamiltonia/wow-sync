//! Bundled brand art (D-22). The ICO holds PNG-compressed entries; the tray picks the 32 px one.
//! Decoding happens once, on first use, and the result is cached.

use std::sync::OnceLock;

pub const TRAY_SIZE: u32 = 32;
const ICO: &[u8] = include_bytes!("../../../assets/brand/wow_addon_sync.ico");

fn le16(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn le32(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// Decodes the `size` x `size` PNG entry of an ICO into RGBA8 pixels.
pub fn decode_ico_entry(ico: &[u8], size: u32) -> Result<Vec<u8>, String> {
    if le16(ico, 0) != Some(0) || le16(ico, 2) != Some(1) {
        return Err("not an ICO file".into());
    }
    let count = le16(ico, 4).ok_or("truncated ICO header")?;
    for i in 0..count {
        let e = 6 + i * 16;
        let (w, h) = (
            *ico.get(e).ok_or("truncated ICO directory")?,
            *ico.get(e + 1).ok_or("truncated ICO directory")?,
        );
        // A stored 0 means 256.
        let dim = |v: u8| if v == 0 { 256 } else { u32::from(v) };
        if dim(w) != size || dim(h) != size {
            continue;
        }
        let len = le32(ico, e + 8).ok_or("truncated ICO directory")?;
        let off = le32(ico, e + 12).ok_or("truncated ICO directory")?;
        let data = ico
            .get(off..off.checked_add(len).ok_or("bad ICO entry range")?)
            .ok_or("ICO entry outside file")?;
        return decode_png_rgba(data, size);
    }
    Err(format!("no {size}px entry in ICO"))
}

fn decode_png_rgba(data: &[u8], size: u32) -> Result<Vec<u8>, String> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(data))
        .read_info()
        .map_err(|e| format!("PNG header: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size().ok_or("PNG too large")?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG data: {e}"))?;
    if (info.width, info.height) != (size, size) || info.bit_depth != png::BitDepth::Eight {
        return Err("unexpected PNG size or depth".into());
    }
    buf.truncate(info.buffer_size());
    match info.color_type {
        png::ColorType::Rgba => Ok(buf),
        png::ColorType::Rgb => Ok(buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 0xff])
            .collect()),
        other => Err(format!("unsupported PNG colour type {other:?}")),
    }
}

/// The tray-sized brand art, RGBA8.
pub fn tray_base() -> &'static [u8] {
    static BASE: OnceLock<Vec<u8>> = OnceLock::new();
    BASE.get_or_init(|| decode_ico_entry(ICO, TRAY_SIZE).expect("bundled brand icon decodes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_art_decodes_to_tray_size_with_visible_pixels() {
        let px = decode_ico_entry(ICO, TRAY_SIZE).expect("decode");
        assert_eq!(px.len(), (TRAY_SIZE * TRAY_SIZE * 4) as usize);
        let visible = px.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
        assert!(visible > 100, "only {visible} visible pixels");
        assert_eq!(tray_base(), px.as_slice());
    }

    #[test]
    fn corrupt_art_is_rejected() {
        assert!(decode_ico_entry(&[0u8; 64], TRAY_SIZE).is_err());
        assert!(decode_ico_entry(b"", TRAY_SIZE).is_err());
        // Damage only the 32 px entry's own bytes, wherever it sits in the file.
        let entry = (0..le16(ICO, 4).unwrap())
            .map(|i| 6 + i * 16)
            .find(|&e| ICO[e] == 32)
            .unwrap();
        let (len, off) = (
            le32(ICO, entry + 8).unwrap(),
            le32(ICO, entry + 12).unwrap(),
        );
        let mut bad = ICO.to_vec();
        bad.truncate(off + len / 2);
        assert!(decode_ico_entry(&bad, TRAY_SIZE).is_err());
        let mut mangled = ICO.to_vec();
        for b in &mut mangled[off + 8..off + len] {
            *b = 0xA5;
        }
        assert!(decode_ico_entry(&mangled, TRAY_SIZE).is_err());
    }
}
