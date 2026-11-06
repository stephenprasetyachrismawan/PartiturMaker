//! Pembaca TrueType minimal: lebar huruf untuk tata letak dan data untuk PDF.
//! Font yang dipakai: Liberation Serif (SIL OFL 1.1), sama ukurannya dengan
//! Times New Roman.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Style {
    Regular,
    Bold,
    Italic,
}

pub struct Font {
    pub data: &'static [u8],
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    pub cap_height: i16,
    pub bbox: [i16; 4],
    advances: Vec<u16>,
    segments: Vec<Seg>,
    glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone, Copy)]
struct Seg {
    start: u16,
    end: u16,
    delta: i16,
    range_offset: u16,
    index: usize,
}

const REGULAR: &[u8] = include_bytes!("../fonts/LiberationSerif-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/LiberationSerif-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../fonts/LiberationSerif-Italic.ttf");

pub fn font(style: Style) -> &'static Font {
    static R: OnceLock<Font> = OnceLock::new();
    static B: OnceLock<Font> = OnceLock::new();
    static I: OnceLock<Font> = OnceLock::new();
    match style {
        Style::Regular => R.get_or_init(|| Font::parse(REGULAR)),
        Style::Bold => B.get_or_init(|| Font::parse(BOLD)),
        Style::Italic => I.get_or_init(|| Font::parse(ITALIC)),
    }
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}
fn i16_at(d: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

impl Font {
    fn parse(data: &'static [u8]) -> Font {
        let num = u16_at(data, 4) as usize;
        let mut tables = std::collections::HashMap::new();
        for i in 0..num {
            let r = 12 + 16 * i;
            let tag = &data[r..r + 4];
            tables.insert(
                tag.to_vec(),
                (u32_at(data, r + 8) as usize, u32_at(data, r + 12) as usize),
            );
        }
        let t = |name: &[u8]| tables.get(name).copied().expect("tabel font hilang");
        let (head, _) = t(b"head");
        let units_per_em = u16_at(data, head + 18);
        let bbox = [
            i16_at(data, head + 36),
            i16_at(data, head + 38),
            i16_at(data, head + 40),
            i16_at(data, head + 42),
        ];
        let (hhea, _) = t(b"hhea");
        let ascent = i16_at(data, hhea + 4);
        let descent = i16_at(data, hhea + 6);
        let n_metrics = u16_at(data, hhea + 34) as usize;
        let (maxp, _) = t(b"maxp");
        let n_glyphs = u16_at(data, maxp + 4) as usize;
        let (hmtx, _) = t(b"hmtx");
        let mut advances = Vec::with_capacity(n_glyphs);
        for g in 0..n_glyphs {
            let m = g.min(n_metrics - 1);
            advances.push(u16_at(data, hmtx + 4 * m));
        }
        let cap_height = tables
            .get(&b"OS/2".to_vec())
            .map(|&(o, len)| {
                if len >= 90 && u16_at(data, o) >= 2 {
                    i16_at(data, o + 88)
                } else {
                    (units_per_em as f32 * 0.66) as i16
                }
            })
            .unwrap_or((units_per_em as f32 * 0.66) as i16);

        // cmap format 4 (Windows Unicode BMP).
        let (cmap, _) = t(b"cmap");
        let n_sub = u16_at(data, cmap + 2) as usize;
        let mut sub = None;
        for i in 0..n_sub {
            let r = cmap + 4 + 8 * i;
            let (pid, eid, off) = (
                u16_at(data, r),
                u16_at(data, r + 2),
                u32_at(data, r + 4) as usize,
            );
            if pid == 3 && eid == 1 && u16_at(data, cmap + off) == 4 {
                sub = Some(cmap + off);
            }
        }
        let sub = sub.expect("cmap format 4 tidak ada");
        let seg_x2 = u16_at(data, sub + 6) as usize;
        let segs = seg_x2 / 2;
        let ends = sub + 14;
        let starts = ends + seg_x2 + 2;
        let deltas = starts + seg_x2;
        let ranges = deltas + seg_x2;
        let mut segments = Vec::with_capacity(segs);
        for s in 0..segs {
            segments.push(Seg {
                end: u16_at(data, ends + 2 * s),
                start: u16_at(data, starts + 2 * s),
                delta: i16_at(data, deltas + 2 * s),
                range_offset: u16_at(data, ranges + 2 * s),
                index: s,
            });
        }
        // Simpan glyphIdArray sebagai salinan agar pencarian sederhana.
        let glyph_area = ranges + seg_x2;
        let sub_len = u16_at(data, sub + 2) as usize;
        let mut glyph_ids = Vec::new();
        let mut o = glyph_area;
        while o + 1 < sub + sub_len {
            glyph_ids.push(u16_at(data, o));
            o += 2;
        }
        Font {
            data,
            units_per_em,
            ascent,
            descent,
            cap_height,
            bbox,
            advances,
            segments,
            glyph_ids,
        }
    }

    pub fn glyph(&self, c: char) -> u16 {
        let cp = c as u32;
        if cp > 0xFFFF {
            return 0;
        }
        let cp = cp as u16;
        let segs = &self.segments;
        let idx = segs.partition_point(|s| s.end < cp);
        let Some(s) = segs.get(idx) else { return 0 };
        if cp < s.start {
            return 0;
        }
        if s.range_offset == 0 {
            return (cp as i32 + s.delta as i32).rem_euclid(65536) as u16;
        }
        // Posisi relatif terhadap glyphIdArray.
        let n = segs.len();
        let k = s.range_offset as usize / 2 + (cp - s.start) as usize - (n - s.index);
        match self.glyph_ids.get(k) {
            Some(&0) | None => 0,
            Some(&g) => (g as i32 + s.delta as i32).rem_euclid(65536) as u16,
        }
    }

    /// Lebar satu huruf dalam satuan em/1000.
    pub fn advance_1000(&self, c: char) -> f32 {
        let g = self.glyph(c) as usize;
        let a = self.advances.get(g).copied().unwrap_or(self.advances[0]);
        a as f32 * 1000.0 / self.units_per_em as f32
    }

    pub fn width(&self, text: &str, size: f32) -> f32 {
        text.chars().map(|c| self.advance_1000(c)).sum::<f32>() * size / 1000.0
    }

    pub fn cap(&self, size: f32) -> f32 {
        self.cap_height as f32 * size / self.units_per_em as f32
    }
}

pub fn text_width(text: &str, size: f32, style: Style) -> f32 {
    font(style).width(text, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lebar_masuk_akal() {
        let f = font(Style::Regular);
        // Times: angka 500/1000, spasi 250/1000.
        assert!((f.advance_1000('5') - 500.0).abs() < 1.0);
        assert!((f.advance_1000(' ') - 250.0).abs() < 1.0);
        assert!(f.width("Haleluya", 10.0) > 30.0);
        assert!(f.glyph('é') != 0);
        assert!(f.cap(1000.0) > 600.0 && f.cap(1000.0) < 700.0);
    }
}
