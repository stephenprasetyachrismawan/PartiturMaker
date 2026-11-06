//! Halaman -> SVG.

use crate::font::Style;
use crate::layout::{Anchor, HitKind, Page, Prim, Seg};
use std::fmt::Write;

pub const FONT_STACK: &str = "'PM Serif', 'Liberation Serif', 'Times New Roman', Times, serif";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn r(v: f32) -> String {
    let s = format!("{:.2}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

pub fn path_d(d: &[Seg]) -> String {
    let mut s = String::new();
    for seg in d {
        match *seg {
            Seg::M(x, y) => write!(s, "M{} {}", r(x), r(y)).unwrap(),
            Seg::L(x, y) => write!(s, "L{} {}", r(x), r(y)).unwrap(),
            Seg::Q(a, b, x, y) => write!(s, "Q{} {} {} {}", r(a), r(b), r(x), r(y)).unwrap(),
            Seg::Z => s.push('Z'),
        }
    }
    s
}

/// `with_hits`: sertakan kotak klik untuk editor.
pub fn render(page: &Page, with_hits: bool) -> String {
    let mut s = String::with_capacity(16 * 1024);
    write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"pm-page\" width=\"{w}pt\" height=\"{h}pt\" viewBox=\"0 0 {w} {h}\" font-family=\"{f}\" style=\"font-kerning:none\">",
        w = r(page.width),
        h = r(page.height),
        f = FONT_STACK
    )
    .unwrap();
    s.push_str(
        "<rect class=\"paper\" width=\"100%\" height=\"100%\" fill=\"#fff\"/><g fill=\"#111\">",
    );
    for p in &page.prims {
        match p {
            Prim::Text { x, y, s: t, size, style, anchor, faint } => {
                write!(s, "<text x=\"{}\" y=\"{}\" font-size=\"{}\"", r(*x), r(*y), r(*size)).unwrap();
                match style {
                    Style::Bold => s.push_str(" font-weight=\"bold\""),
                    Style::Italic => s.push_str(" font-style=\"italic\""),
                    Style::Regular => {}
                }
                match anchor {
                    Anchor::Middle => s.push_str(" text-anchor=\"middle\""),
                    Anchor::End => s.push_str(" text-anchor=\"end\""),
                    Anchor::Start => {}
                }
                if *faint {
                    s.push_str(" class=\"faint\" fill=\"#8a948f\"");
                }
                write!(s, " xml:space=\"preserve\">{}</text>", esc(t)).unwrap();
            }
            Prim::Rect { x, y, w, h } => write!(s, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>", r(*x), r(*y), r(*w), r(*h)).unwrap(),
            Prim::Line { x1, y1, x2, y2, w } => write!(
                s,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                r(*x1),
                r(*y1),
                r(*x2),
                r(*y2),
                r(*w)
            )
            .unwrap(),
            Prim::Circle { x, y, r: rad } => write!(s, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>", r(*x), r(*y), r(*rad)).unwrap(),
            Prim::Path { d, stroke } => match stroke {
                Some(w) => write!(s, "<path d=\"{}\" fill=\"none\" stroke=\"#111\" stroke-width=\"{}\"/>", path_d(d), r(*w)).unwrap(),
                None => write!(s, "<path d=\"{}\"/>", path_d(d)).unwrap(),
            },
        }
    }
    s.push_str("</g>");
    if with_hits {
        s.push_str("<g class=\"hits\" fill=\"transparent\">");
        for h in &page.hits {
            let attrs = match &h.kind {
                HitKind::Note { voice, index } => {
                    format!("data-k=\"note\" data-v=\"{}\" data-i=\"{}\"", voice, index)
                }
                HitKind::Bar { voice, index } => {
                    format!("data-k=\"bar\" data-v=\"{}\" data-i=\"{}\"", voice, index)
                }
                HitKind::End { voice, index } => {
                    format!("data-k=\"end\" data-v=\"{}\" data-i=\"{}\"", voice, index)
                }
                HitKind::Syllable { verse, slot } => format!(
                    "data-k=\"syl\" data-verse=\"{}\" data-slot=\"{}\"",
                    verse, slot
                ),
                HitKind::Meta { key } => format!("data-k=\"meta\" data-key=\"{}\"", esc(key)),
                HitKind::Text { number } => format!("data-k=\"text\" data-number=\"{}\"", number),
            };
            write!(
                s,
                "<rect class=\"hit\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                attrs,
                r(h.x),
                r(h.y),
                r(h.w),
                r(h.h)
            )
            .unwrap();
        }
        s.push_str("</g>");
    }
    s.push_str("</svg>");
    s
}
