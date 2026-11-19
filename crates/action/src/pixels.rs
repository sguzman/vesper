//! An intentionally tiny source-art format: dimensions, RGBA palette, then exact pixels.
//! All sprite art is external and can be edited without recompiling the engine.

use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct PixelSource {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl PixelSource {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut lines = source
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'));
        let size = lines.next().ok_or("missing dimensions")?;
        let dims = size.split_whitespace().collect::<Vec<_>>();
        if dims.len() != 2 {
            return Err("expected WIDTH HEIGHT".into());
        }
        let width = dims[0].parse::<u32>().map_err(|_| "invalid width")?;
        let height = dims[1].parse::<u32>().map_err(|_| "invalid height")?;
        if width == 0 || height == 0 || width > 1024 || height > 1024 {
            return Err("pixel-source dimensions must be 1..=1024".into());
        }
        let mut colors: HashMap<char, [u8; 4]> = HashMap::new();
        let mut found_separator = false;
        for line in lines.by_ref() {
            if line == "--" {
                found_separator = true;
                break;
            }
            let Some((symbol, hex)) = line.split_once(' ') else {
                return Err(format!("invalid palette entry: {line}"));
            };
            let mut chars = symbol.chars();
            let Some(symbol) = chars.next() else {
                return Err("missing palette symbol".into());
            };
            if chars.next().is_some() || !symbol.is_ascii() {
                return Err("palette symbol must be one ASCII character".into());
            }
            let hex = hex.trim();
            if hex.len() != 6 && hex.len() != 8 {
                return Err("palette colors must use RRGGBB or RRGGBBAA".into());
            }
            let value = u32::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid RGBA color: {hex}"))?;
            let color = if hex.len() == 6 {
                [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
            } else {
                [(value >> 24) as u8, (value >> 16) as u8, (value >> 8) as u8, value as u8]
            };
            if colors.insert(symbol, color).is_some() {
                return Err(format!("duplicate palette symbol: {symbol}"));
            }
        }
        if !found_separator {
            return Err("pixel source missing -- separator".into());
        }
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            let row = lines.next().ok_or(format!("missing pixel row {y}"))?;
            if row.len() != width as usize || !row.is_ascii() {
                return Err(format!("pixel row {y} does not match width {width}"));
            }
            for symbol in row.chars() {
                rgba.extend_from_slice(
                    colors.get(&symbol).ok_or(format!("unknown palette pixel {symbol:?}"))?,
                );
            }
        }
        if lines.next().is_some() {
            return Err("unexpected extra pixel rows".into());
        }
        Ok(Self { width, height, rgba })
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Self::parse(&raw).map_err(|error| format!("{}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_editable_rgba_pixels() {
        let art = PixelSource::parse(
            "# comment\n2 2\nA ff8040\n_ 00000000\n--\nA_\n_A\n",
        )
        .unwrap();
        assert_eq!((art.width, art.height), (2, 2));
        assert_eq!(&art.rgba[..8], &[255, 128, 64, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn rejects_malformed_art_instead_of_corrupting_graphics() {
        assert!(PixelSource::parse("3 1\nA ffffff\n--\nAA\n").is_err());
        assert!(PixelSource::parse("1 1\nA ffffff\n--\nZ\n").is_err());
        assert!(PixelSource::parse("1 1\nA ffffff\nA aaaaaa\n--\nA\n").is_err());
    }
}
