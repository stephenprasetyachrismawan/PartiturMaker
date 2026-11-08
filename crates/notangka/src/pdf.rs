//! Halaman -> PDF. Font Liberation Serif ditanam (TrueType, WinAnsi).

use crate::font::{font, text_width, Style};
use crate::layout::{Anchor, Page, Prim, Seg};
use std::fmt::Write as _;

const WINANSI_HIGH: [(u8, char); 27] = [
    (0x80, '€'),
    (0x82, '‚'),
    (0x83, 'ƒ'),
    (0x84, '„'),
    (0x85, '…'),
    (0x86, '†'),
    (0x87, '‡'),
    (0x88, 'ˆ'),
    (0x89, '‰'),
    (0x8A, 'Š'),
    (0x8B, '‹'),
    (0x8C, 'Œ'),
    (0x8E, 'Ž'),
    (0x91, '‘'),
    (0x92, '’'),
    (0x93, '“'),
    (0x94, '”'),
    (0x95, '•'),
    (0x96, '–'),
    (0x97, '—'),
    (0x98, '˜'),
    (0x99, '™'),
    (0x9A, 'š'),
    (0x9B, '›'),
    (0x9C, 'œ'),
    (0x9E, 'ž'),
    (0x9F, 'Ÿ'),
];

fn to_winansi(c: char) -> u8 {
    let u = c as u32;
    if (0x20..0x7F).contains(&u) || (0xA0..=0xFF).contains(&u) {
        return u as u8;
    }
    WINANSI_HIGH
        .iter()
        .find(|(_, ch)| *ch == c)
        .map(|(b, _)| *b)
        .unwrap_or(b'?')
}

fn from_winansi(b: u8) -> Option<char> {
    match b {
        0x20..=0x7E | 0xA0..=0xFF => Some(b as char),
        _ => WINANSI_HIGH.iter().find(|(x, _)| *x == b).map(|(_, c)| *c),
    }
}

fn pdf_string(s: &str) -> String {
    let mut o = String::from("(");
    for c in s.chars() {
        let b = to_winansi(if c == '~' { ' ' } else { c });
        match b {
            b'(' | b')' | b'\\' => {
                o.push('\\');
                o.push(b as char);
            }
            0x20..=0x7E => o.push(b as char),
            _ => write!(o, "\\{:03o}", b).unwrap(),
        }
    }
    o.push(')');
    o
}

fn n(v: f32) -> String {
    let s = format!("{:.3}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

struct Writer {
    buf: Vec<u8>,
    offsets: Vec<usize>,
}

impl Writer {
    fn obj(&mut self, id: usize, body: &[u8]) {
        if self.offsets.len() <= id {
            self.offsets.resize(id + 1, 0);
        }
        self.offsets[id] = self.buf.len();
        self.buf
            .extend_from_slice(format!("{} 0 obj\n", id).as_bytes());
        self.buf.extend_from_slice(body);
        self.buf.extend_from_slice(b"\nendobj\n");
    }
    fn stream(&mut self, id: usize, dict: &str, data: &[u8]) {
        let mut body = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        self.obj(id, &body);
    }
}

pub fn render(pages: &[Page], title: &str) -> Vec<u8> {
    let styles = [Style::Regular, Style::Bold, Style::Italic];
    let names = [
        "LiberationSerif",
        "LiberationSerif-Bold",
        "LiberationSerif-Italic",
    ];
    let mut w = Writer {
        buf: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(),
        offsets: vec![0],
    };

    // Nomor objek: 1 katalog, 2 pages, 3 info, 4.. font (3 obj per font), lalu halaman.
    let font_base = 4;
    let page_base = font_base + 3 * styles.len();
    let page_ids: Vec<usize> = (0..pages.len()).map(|i| page_base + 2 * i).collect();

    w.obj(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = page_ids.iter().map(|id| format!("{} 0 R", id)).collect();
    w.obj(
        2,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .as_bytes(),
    );
    w.obj(
        3,
        format!(
            "<< /Title {} /Producer (PartiturMaker) >>",
            pdf_string(title)
        )
        .as_bytes(),
    );

    for (k, st) in styles.iter().enumerate() {
        let f = font(*st);
        let id = font_base + 3 * k;
        let scale = 1000.0 / f.units_per_em as f32;
        let widths: Vec<String> = (32u8..=255)
            .map(|b| {
                from_winansi(b)
                    .map(|c| n(f.advance_1000(c)))
                    .unwrap_or_else(|| "0".into())
            })
            .collect();
        w.obj(
            id,
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /{} /FirstChar 32 /LastChar 255 /Widths [{}] /FontDescriptor {} 0 R /Encoding /WinAnsiEncoding >>",
                names[k],
                widths.join(" "),
                id + 1
            )
            .as_bytes(),
        );
        let flags = 32 + 2 + if *st == Style::Italic { 64 } else { 0 };
        w.obj(
            id + 1,
            format!(
                "<< /Type /FontDescriptor /FontName /{} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /FontFile2 {} 0 R >>",
                names[k],
                flags,
                n(f.bbox[0] as f32 * scale),
                n(f.bbox[1] as f32 * scale),
                n(f.bbox[2] as f32 * scale),
                n(f.bbox[3] as f32 * scale),
                if *st == Style::Italic { -15.5 } else { 0.0 },
                n(f.ascent as f32 * scale),
                n(f.descent as f32 * scale),
                n(f.cap_height as f32 * scale),
                if *st == Style::Bold { 140 } else { 80 },
                id + 2
            )
            .as_bytes(),
        );
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(f.data, 6);
        w.stream(
            id + 2,
            &format!("/Filter /FlateDecode /Length1 {}", f.data.len()),
            &packed,
        );
    }

    for (i, p) in pages.iter().enumerate() {
        let id = page_ids[i];
        let content = page_content(p);
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(content.as_bytes(), 6);
        let fonts: Vec<String> = (0..styles.len())
            .map(|k| format!("/F{} {} 0 R", k, font_base + 3 * k))
            .collect();
        w.obj(
            id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {} >> >> /Contents {} 0 R >>",
                n(p.width),
                n(p.height),
                fonts.join(" "),
                id + 1
            )
            .as_bytes(),
        );
        w.stream(id + 1, "/Filter /FlateDecode", &packed);
    }

    let xref = w.buf.len();
    let count = w.offsets.len();
    let mut tail = format!("xref\n0 {}\n0000000000 65535 f \n", count);
    for off in w.offsets.iter().skip(1) {
        write!(tail, "{:010} 00000 n \n", off).unwrap();
    }
    write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{}\n%%EOF\n",
        count, xref
    )
    .unwrap();
    w.buf.extend_from_slice(tail.as_bytes());
    w.buf
}

fn page_content(p: &Page) -> String {
    let h = p.height;
    let mut s = String::new();
    s.push_str("0.067 g 0.067 G 1 J\n");
    let fy = |y: f32| h - y;
    for prim in &p.prims {
        match prim {
            Prim::Text {
                x,
                y,
                s: t,
                size,
                style,
                anchor,
                faint,
            } => {
                if *faint {
                    continue;
                }
                let k = match style {
                    Style::Regular => 0,
                    Style::Bold => 1,
                    Style::Italic => 2,
                };
                let tw = text_width(t, *size, *style);
                let x = match anchor {
                    Anchor::Start => *x,
                    Anchor::Middle => x - tw / 2.0,
                    Anchor::End => x - tw,
                };
                writeln!(
                    s,
                    "BT /F{} {} Tf 1 0 0 1 {} {} Tm {} Tj ET",
                    k,
                    n(*size),
                    n(x),
                    n(fy(*y)),
                    pdf_string(t)
                )
                .unwrap();
            }
            Prim::Rect { x, y, w, h: rh } => {
                writeln!(s, "{} {} {} {} re f", n(*x), n(fy(y + rh)), n(*w), n(*rh)).unwrap();
            }
            Prim::Line { x1, y1, x2, y2, w } => {
                writeln!(
                    s,
                    "{} w {} {} m {} {} l S",
                    n(*w),
                    n(*x1),
                    n(fy(*y1)),
                    n(*x2),
                    n(fy(*y2))
                )
                .unwrap();
            }
            Prim::Circle { x, y, r } => {
                let k = 0.5523 * r;
                let (cx, cy) = (*x, fy(*y));
                writeln!(
                    s,
                    "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c f",
                    n(cx + r), n(cy),
                    n(cx + r), n(cy + k), n(cx + k), n(cy + r), n(cx), n(cy + r),
                    n(cx - k), n(cy + r), n(cx - r), n(cy + k), n(cx - r), n(cy),
                    n(cx - r), n(cy - k), n(cx - k), n(cy - r), n(cx), n(cy - r),
                    n(cx + k), n(cy - r), n(cx + r), n(cy - k), n(cx + r), n(cy)
                )
                .unwrap();
            }
            Prim::Path { d, stroke } => {
                let mut cur = (0.0f32, 0.0f32);
                for seg in d {
                    match *seg {
                        Seg::M(x, y) => {
                            write!(s, "{} {} m ", n(x), n(fy(y))).unwrap();
                            cur = (x, y);
                        }
                        Seg::L(x, y) => {
                            write!(s, "{} {} l ", n(x), n(fy(y))).unwrap();
                            cur = (x, y);
                        }
                        Seg::Q(qx, qy, x, y) => {
                            let c1 = (
                                cur.0 + 2.0 / 3.0 * (qx - cur.0),
                                cur.1 + 2.0 / 3.0 * (qy - cur.1),
                            );
                            let c2 = (x + 2.0 / 3.0 * (qx - x), y + 2.0 / 3.0 * (qy - y));
                            write!(
                                s,
                                "{} {} {} {} {} {} c ",
                                n(c1.0),
                                n(fy(c1.1)),
                                n(c2.0),
                                n(fy(c2.1)),
                                n(x),
                                n(fy(y))
                            )
                            .unwrap();
                            cur = (x, y);
                        }
                        Seg::Z => s.push_str("h "),
                    }
                }
                match stroke {
                    Some(w) => writeln!(s, "{} w S", n(*w)).unwrap(),
                    None => s.push_str("f\n"),
                }
            }
        }
    }
    s
}
