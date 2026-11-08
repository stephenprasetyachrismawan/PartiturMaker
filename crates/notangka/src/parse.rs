//! Parser bahasa Nada (berkas `.nada`).
//!
//! ```text
//! // komentar
//! nomor = "KJ 1"
//! judul = "Haleluya! Pujilah"
//! nada  = F
//! ketuk = 4
//!
//! suara S {
//!   3 [5 6] 3 [1 2] | 3 [5 6] 3 . ; |
//! }
//!
//! bait 1 {
//!   Ha-le-lu-ya! Pu-ji-lah Al-lah yang A-gung
//! }
//! ```

use crate::model::*;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub hint: Option<String>,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

pub struct Parsed {
    pub doc: Doc,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parsed {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

struct Ctx<'a> {
    src: &'a str,
    diags: Vec<Diagnostic>,
}

impl<'a> Ctx<'a> {
    fn push(&mut self, sev: Severity, span: Span, msg: impl Into<String>, hint: Option<String>) {
        let (line, col) = line_col(self.src, span.start);
        self.diags.push(Diagnostic {
            severity: sev,
            message: msg.into(),
            hint,
            start: span.start,
            end: span
                .end
                .max(span.start + 1)
                .min(self.src.len().max(span.start + 1)),
            line,
            col,
        });
    }
    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Severity::Error, span, msg, None);
    }
    fn error_hint(&mut self, span: Span, msg: impl Into<String>, hint: impl Into<String>) {
        self.push(Severity::Error, span, msg, Some(hint.into()));
    }
    fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Severity::Warning, span, msg, None);
    }
}

/// Baris dan kolom (mulai 1) dari posisi byte.
pub fn line_col(src: &str, pos: usize) -> (usize, usize) {
    let pos = pos.min(src.len());
    let before = &src[..floor_char(src, pos)];
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map(|s| s.chars().count())
        .unwrap_or(0)
        + 1;
    (line, col)
}

fn floor_char(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn canonical_meta_key(k: &str) -> Option<&'static str> {
    Some(match k {
        "nomor" | "no" => "nomor",
        "judul" => "judul",
        "ayat" | "subjudul" => "ayat",
        "nada" | "do" => "nada",
        "ketuk" | "birama" => "ketuk",
        "tempo" => "tempo",
        "bpm" => "bpm",
        "syair" | "teks_oleh" => "syair",
        "lagu" | "musik" => "lagu",
        "terjemahan" => "terjemahan",
        "aransemen" => "aransemen",
        "catatan" | "hakcipta" => "catatan",
        _ => return None,
    })
}

pub fn parse(src: &str) -> Parsed {
    let mut cx = Ctx {
        src,
        diags: Vec::new(),
    };
    let mut doc = Doc::default();
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut seen_item = false;

    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = find_eol(b, i);
            if !seen_item {
                doc.comments.push(src[i + 2..end].trim().to_string());
            }
            i = end;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            seen_item = true;
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = src[start..i].to_ascii_lowercase();
            match word.as_str() {
                "suara" => i = parse_voice_block(&mut cx, &mut doc, i, start),
                "bait" | "teks" => {
                    i = parse_verse_block(&mut cx, &mut doc, i, start, word == "teks")
                }
                _ => i = parse_meta(&mut cx, &mut doc, i, start, &word),
            }
            continue;
        }
        let end = find_eol(b, i);
        cx.error_hint(
            Span::new(i, i + 1),
            format!("Tidak dimengerti: \"{}\"", src[i..end].trim()),
            "Setiap baris dimulai dengan kunci (judul = ...), suara S { ... } atau bait 1 { ... }",
        );
        i = end;
    }

    check_semantics(&mut cx, &doc);
    let mut diagnostics = cx.diags;
    diagnostics.sort_by_key(|d| (d.start, d.severity == Severity::Warning));
    Parsed { doc, diagnostics }
}

fn find_eol(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_inline_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\r') {
        i += 1;
    }
    i
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn parse_meta(cx: &mut Ctx, doc: &mut Doc, mut i: usize, start: usize, word: &str) -> usize {
    let src = cx.src;
    let b = src.as_bytes();
    let key_span = Span::new(start, i);
    i = skip_inline_ws(b, i);
    if b.get(i) != Some(&b'=') {
        let end = find_eol(b, i);
        cx.error_hint(
            key_span,
            format!("\"{}\" harus diikuti tanda =", &src[start..key_span.end]),
            "Contoh: judul = \"Haleluya! Pujilah\"",
        );
        return end;
    }
    i += 1;
    i = skip_inline_ws(b, i);
    let (value, end) = if b.get(i) == Some(&b'"') {
        let mut s = String::new();
        let mut j = i + 1;
        let mut closed = false;
        while j < b.len() && b[j] != b'\n' {
            let ch = src[j..].chars().next().unwrap();
            if ch == '\\' && j + 1 < b.len() {
                let nx = src[j + 1..].chars().next().unwrap();
                s.push(match nx {
                    'n' => '\n',
                    other => other,
                });
                j += 1 + nx.len_utf8();
                continue;
            }
            if ch == '"' {
                closed = true;
                j += 1;
                break;
            }
            s.push(ch);
            j += ch.len_utf8();
        }
        if !closed {
            cx.error_hint(
                Span::new(i, j),
                "Tanda petik belum ditutup",
                "Akhiri teks dengan \"",
            );
        }
        let rest = skip_inline_ws(b, j);
        let mut end = rest;
        if rest < b.len()
            && b[rest] != b'\n'
            && !(b[rest] == b'/' && b.get(rest + 1) == Some(&b'/'))
        {
            end = find_eol(b, rest);
            cx.error(
                Span::new(rest, end),
                "Ada teks lebih sesudah tanda petik penutup",
            );
        }
        (s, end.max(j))
    } else {
        let end = find_eol(b, i);
        let mut raw = &src[i..end];
        if let Some(p) = raw.find("//") {
            raw = &raw[..p];
        }
        (raw.trim().to_string(), end)
    };

    match canonical_meta_key(word) {
        Some(k) => {
            if k == "bpm" && !value.is_empty() && value.parse::<u32>().is_err() {
                cx.error_hint(key_span, "bpm harus berupa angka", "Contoh: bpm = 90");
            }
            if k == "ketuk" && !value.is_empty() {
                let ok = value
                    .split('/')
                    .all(|p| p.trim().parse::<u32>().map(|n| n > 0).unwrap_or(false));
                if !ok {
                    cx.error_hint(
                        key_span,
                        "ketuk harus berupa angka",
                        "Contoh: ketuk = 4 atau ketuk = 6/8",
                    );
                }
            }
            doc.meta.set(k, Some(value));
        }
        None => cx.error_hint(
            key_span,
            format!("Kunci \"{}\" tidak dikenal", word),
            format!("Kunci yang tersedia: {}", META_KEYS.join(", ")),
        ),
    }
    end
}

fn read_block(cx: &mut Ctx, mut i: usize, head: Span) -> Option<(usize, usize, usize)> {
    let b = cx.src.as_bytes();
    i = skip_ws(b, i);
    if b.get(i) != Some(&b'{') {
        cx.error_hint(
            head,
            "Kurung kurawal { tidak ditemukan",
            "Contoh: suara S { 1 2 3 4 | }",
        );
        return None;
    }
    let open = i + 1;
    let mut j = open;
    while j < b.len() && b[j] != b'}' {
        if b[j] == b'{' {
            cx.error(
                Span::new(j, j + 1),
                "Kurung { di dalam blok tidak diperbolehkan",
            );
        }
        j += 1;
    }
    if j >= b.len() {
        cx.error_hint(head, "Blok belum ditutup", "Tambahkan } di akhir blok");
        return Some((open, b.len(), b.len()));
    }
    Some((open, j, j + 1))
}

fn parse_voice_block(cx: &mut Ctx, doc: &mut Doc, mut i: usize, start: usize) -> usize {
    let b = cx.src.as_bytes();
    i = skip_inline_ws(b, i);
    let ns = i;
    while i < b.len() && (b[i].is_ascii_alphanumeric()) {
        i += 1;
    }
    let raw_name = &cx.src[ns..i];
    let head = Span::new(start, i);
    let name = match voice_name(raw_name) {
        Some(n) => n,
        None => {
            cx.error_hint(
                Span::new(ns, i.max(ns + 1)),
                if raw_name.is_empty() {
                    "Nama suara belum ditulis".to_string()
                } else {
                    format!("Suara \"{}\" tidak dikenal", raw_name)
                },
                "Pilih S (sopran), A (alto), T (tenor) atau B (bas)",
            );
            "S"
        }
    };
    let Some((open, close, after)) = read_block(cx, i, head) else {
        return find_eol(b, i);
    };
    let items = parse_music(cx, open, close);
    if let Some(v) = doc.voices.iter_mut().find(|v| v.name == name) {
        v.items.extend(items);
    } else {
        doc.voices.push(Voice {
            name: name.to_string(),
            items,
        });
        doc.voices
            .sort_by_key(|v| VOICE_ORDER.iter().position(|n| *n == v.name).unwrap_or(9));
    }
    after
}

fn parse_verse_block(
    cx: &mut Ctx,
    doc: &mut Doc,
    mut i: usize,
    start: usize,
    as_text: bool,
) -> usize {
    let b = cx.src.as_bytes();
    i = skip_inline_ws(b, i);
    let ns = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let head = Span::new(start, i);
    let number = match cx.src[ns..i].parse::<u32>() {
        Ok(n) => n,
        Err(_) => {
            cx.error_hint(head, "Nomor bait belum ditulis", "Contoh: bait 1 { ... }");
            (doc.verses.len() + doc.texts.len() + 1) as u32
        }
    };
    let Some((open, close, after)) = read_block(cx, i, head) else {
        return find_eol(b, i);
    };
    let body = &cx.src[open..close];
    if as_text {
        let lines: Vec<String> = body.lines().map(|l| l.trim().to_string()).collect();
        let first = lines
            .iter()
            .position(|l| !l.is_empty())
            .unwrap_or(lines.len());
        let last = lines
            .iter()
            .rposition(|l| !l.is_empty())
            .map(|p| p + 1)
            .unwrap_or(first);
        let lines = lines[first..last].to_vec();
        if let Some(t) = doc.texts.iter_mut().find(|t| t.number == number) {
            t.lines.extend(lines);
        } else {
            doc.texts.push(TextVerse { number, lines });
            doc.texts.sort_by_key(|t| t.number);
        }
    } else {
        let syl = parse_lyrics(body);
        if let Some(v) = doc.verses.iter_mut().find(|v| v.number == number) {
            v.syllables.extend(syl);
        } else {
            doc.verses.push(Verse {
                number,
                syllables: syl,
            });
            doc.verses.sort_by_key(|v| v.number);
        }
    }
    after
}

/// Pecah teks lirik menjadi suku kata.
pub fn parse_lyrics(body: &str) -> Vec<Option<Syllable>> {
    let mut out = Vec::new();
    for word in body.split_whitespace() {
        if word == "_" {
            out.push(None);
            continue;
        }
        let parts: Vec<&str> = word.split('-').collect();
        let n = parts.len();
        for (k, p) in parts.iter().enumerate() {
            if p.is_empty() {
                continue;
            }
            if *p == "_" {
                out.push(None);
                continue;
            }
            // "le-" di akhir baris tetap bersambung ke baris berikutnya.
            out.push(Some(Syllable {
                text: p.to_string(),
                hyphen: k + 1 < n,
            }));
        }
    }
    out
}

// ---------- Musik ----------

#[derive(Debug, Clone)]
enum TokKind {
    Note { pitch: Pitch, fermata: bool },
    Open,
    Close,
    SlurOpen,
    SlurClose,
    Bar(BarKind),
    Breath,
    Break,
    Label(String),
}

#[derive(Debug, Clone)]
struct Tok {
    kind: TokKind,
    span: Span,
}

fn lex_music(cx: &mut Ctx, from: usize, to: usize) -> Vec<Tok> {
    let src = cx.src;
    let b = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = from;
    while i < to {
        let c = b[i];
        let s = i;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            i = find_eol(b, i).min(to);
            continue;
        }
        let tk = match c {
            b'[' => {
                i += 1;
                TokKind::Open
            }
            b']' => {
                i += 1;
                TokKind::Close
            }
            b'(' => {
                i += 1;
                TokKind::SlurOpen
            }
            b')' => {
                i += 1;
                TokKind::SlurClose
            }
            b';' => {
                i += 1;
                TokKind::Breath
            }
            b'\\' => {
                i += 1;
                TokKind::Break
            }
            b'|' => {
                let (k, n) = match (b.get(i + 1), b.get(i + 2)) {
                    (Some(b'|'), _) => (BarKind::Double, 2),
                    (Some(b'.'), _) => (BarKind::Final, 2),
                    (Some(b':'), _) => (BarKind::RepeatStart, 2),
                    _ => (BarKind::Single, 1),
                };
                i += n;
                TokKind::Bar(k)
            }
            b':' if b.get(i + 1) == Some(&b'|') => {
                if b.get(i + 2) == Some(&b':') {
                    i += 3;
                    TokKind::Bar(BarKind::RepeatBoth)
                } else {
                    i += 2;
                    TokKind::Bar(BarKind::RepeatEnd)
                }
            }
            b'@' => {
                i += 1;
                let text = if b.get(i) == Some(&b'"') {
                    let st = i + 1;
                    let mut j = st;
                    while j < to && b[j] != b'"' && b[j] != b'\n' {
                        j += 1;
                    }
                    let t = src[st..j].to_string();
                    if j < to && b[j] == b'"' {
                        j += 1;
                    } else {
                        cx.error(Span::new(s, j), "Label belum ditutup dengan \"");
                    }
                    i = j;
                    t
                } else {
                    let st = i;
                    while i < to && !b[i].is_ascii_whitespace() && !b"[]()|;\\".contains(&b[i]) {
                        i += 1;
                    }
                    src[st..i].to_string()
                };
                if text.trim().is_empty() {
                    cx.error_hint(
                        Span::new(s, i),
                        "Label kosong",
                        "Contoh: @Refrein atau @\"Ulang 2x\"",
                    );
                }
                TokKind::Label(text)
            }
            b'0'..=b'7' | b'.' => {
                i += 1;
                let mut octave = 0i8;
                let mut acc = 0i8;
                let mut fermata = false;
                while i < to {
                    match b[i] {
                        b'\'' => octave += 1,
                        b',' => octave -= 1,
                        b'#' => acc = 1,
                        b'b' => acc = -1,
                        b'^' => fermata = true,
                        _ => break,
                    }
                    i += 1;
                }
                let pitch = match c {
                    b'.' => Pitch::Hold,
                    b'0' => Pitch::Rest,
                    d => Pitch::Tone {
                        degree: d - b'0',
                        octave,
                        accidental: acc,
                    },
                };
                if !matches!(pitch, Pitch::Tone { .. }) && (octave != 0 || acc != 0) {
                    cx.error(
                        Span::new(s, i),
                        "Titik oktaf, kres dan mol hanya untuk not 1 sampai 7",
                    );
                }
                TokKind::Note { pitch, fermata }
            }
            b'8' | b'9' => {
                i += 1;
                cx.error_hint(
                    Span::new(s, i),
                    "Not angka hanya 1 sampai 7",
                    "Pakai 0 untuk tanda diam",
                );
                continue;
            }
            _ => {
                let ch = src[i..].chars().next().unwrap();
                i += ch.len_utf8();
                let hint = match ch {
                    '-' => "Perpanjangan ketuk ditulis dengan titik: 5 . . .",
                    '/' => "Untuk setengah ketuk, kurung dengan [ ]: [5 6]",
                    '{' | '}' => "Kurung kurawal hanya untuk membuka dan menutup blok",
                    _ => "Lihat panduan penulisan (F1)",
                };
                cx.error_hint(
                    Span::new(s, i),
                    format!("Tanda \"{}\" tidak dikenal di baris not", ch),
                    hint,
                );
                continue;
            }
        };
        toks.push(Tok {
            kind: tk,
            span: Span::new(s, i),
        });
    }
    toks
}

/// Pohon ketuk: daun = not, grup = [ ... ], tanda = token tanpa durasi.
enum Node {
    Leaf(usize),
    Group(Vec<Node>, Span),
    Mark(usize),
}

fn parse_music(cx: &mut Ctx, from: usize, to: usize) -> Vec<Item> {
    let toks = lex_music(cx, from, to);
    let mut pos = 0;
    let nodes = parse_nodes(cx, &toks, &mut pos, None);

    let mut items = Vec::new();
    let mut st = FlatState::default();
    for n in &nodes {
        flatten(cx, &toks, n, BEAT, &mut items, &mut st, true);
    }
    if let Some(sp) = st.open_slur {
        cx.error_hint(
            sp,
            "Legato ( belum ditutup",
            "Tambahkan ) sesudah not terakhir legato",
        );
    }
    if let Some(l) = st.pending_label {
        cx.warn(
            Span::new(to, to),
            format!("Label \"{}\" tidak diikuti not", l),
        );
    }
    items
}

fn parse_nodes(cx: &mut Ctx, toks: &[Tok], pos: &mut usize, open: Option<Span>) -> Vec<Node> {
    let mut out = Vec::new();
    while *pos < toks.len() {
        let t = &toks[*pos];
        match t.kind {
            TokKind::Open => {
                let sp = t.span;
                *pos += 1;
                let children = parse_nodes(cx, toks, pos, Some(sp));
                out.push(Node::Group(children, sp));
            }
            TokKind::Close => {
                if open.is_some() {
                    *pos += 1;
                    return out;
                }
                cx.error(t.span, "Kurung ] tanpa pasangan [");
                *pos += 1;
            }
            TokKind::Note { .. } => {
                out.push(Node::Leaf(*pos));
                *pos += 1;
            }
            _ => {
                out.push(Node::Mark(*pos));
                *pos += 1;
            }
        }
    }
    if let Some(sp) = open {
        cx.error_hint(
            sp,
            "Kurung [ belum ditutup",
            "Setiap [ perlu ] , mis. [5 6]",
        );
    }
    out
}

#[derive(Default)]
struct FlatState {
    pending_slur: Option<Span>,
    open_slur: Option<Span>,
    pending_label: Option<String>,
}

fn last_tone(items: &mut [Item]) -> Option<&mut Note> {
    items.iter_mut().rev().find_map(|i| match i {
        Item::Note(n) if n.is_tone() => Some(n),
        _ => None,
    })
}

fn last_note(items: &mut [Item]) -> Option<&mut Note> {
    items.iter_mut().rev().find_map(|i| match i {
        Item::Note(n) => Some(n),
        _ => None,
    })
}

fn flatten(
    cx: &mut Ctx,
    toks: &[Tok],
    node: &Node,
    dur: u32,
    items: &mut Vec<Item>,
    st: &mut FlatState,
    top: bool,
) {
    match node {
        Node::Leaf(ix) => {
            let t = &toks[*ix];
            if let TokKind::Note { pitch, fermata } = t.kind {
                let mut n = Note::new(pitch, dur);
                n.fermata = fermata;
                n.span = t.span;
                n.label = st.pending_label.take();
                if let Some(sp) = st.pending_slur.take() {
                    if n.is_tone() {
                        n.slur_start = true;
                        st.open_slur = Some(sp);
                    } else {
                        cx.error(sp, "Legato harus dimulai pada not, bukan titik atau 0");
                    }
                }
                if matches!(pitch, Pitch::Hold) && !items.iter().any(|i| matches!(i, Item::Note(_)))
                {
                    cx.error_hint(
                        t.span,
                        "Titik tidak boleh di awal lagu",
                        "Titik memperpanjang not sebelumnya",
                    );
                }
                items.push(Item::Note(n));
            }
        }
        Node::Group(children, sp) => {
            let k = children
                .iter()
                .filter(|c| !matches!(c, Node::Mark(_)))
                .count() as u32;
            if k == 0 {
                cx.error(*sp, "Kurung [ ] kosong");
                return;
            }
            if k == 1 {
                cx.warn(*sp, "Kurung [ ] berisi satu not tidak mengubah panjangnya");
            }
            if dur % k != 0 || dur / k < BEAT / 16 {
                cx.error_hint(
                    *sp,
                    "Pembagian ketuk terlalu halus",
                    "Paling halus 1/16 ketuk",
                );
                return;
            }
            for c in children {
                flatten(cx, toks, c, dur / k, items, st, false);
            }
        }
        Node::Mark(ix) => {
            let t = &toks[*ix];
            match &t.kind {
                TokKind::Bar(k) => {
                    if !top {
                        cx.error_hint(
                            t.span,
                            "Garis birama tidak boleh di dalam [ ]",
                            "Tutup kurung dulu sebelum |",
                        );
                    }
                    items.push(Item::Bar(*k, t.span));
                }
                TokKind::Break => items.push(Item::Break(t.span)),
                TokKind::SlurOpen => {
                    if st.open_slur.is_some() || st.pending_slur.is_some() {
                        cx.error(t.span, "Legato di dalam legato tidak didukung");
                    } else {
                        st.pending_slur = Some(t.span);
                    }
                }
                TokKind::SlurClose => {
                    if st.open_slur.take().is_none() {
                        cx.error(t.span, "Kurung ) tanpa pasangan (");
                    } else if let Some(n) = last_tone(items) {
                        if n.slur_start {
                            n.slur_start = false;
                            cx.warn(t.span, "Legato hanya berisi satu not");
                        } else {
                            n.slur_end = true;
                        }
                    }
                }
                TokKind::Breath => match last_note(items) {
                    Some(n) => n.breath = true,
                    None => cx.error(t.span, "Tanda napas harus sesudah not"),
                },
                TokKind::Label(s) => st.pending_label = Some(s.clone()),
                _ => {}
            }
        }
    }
}

fn check_semantics(cx: &mut Ctx, doc: &Doc) {
    let Some(first) = doc.voices.first() else {
        return;
    };
    let len0 = first.length();
    for v in doc.voices.iter().skip(1) {
        let l = v.length();
        if l != len0 {
            let sp = v.items.iter().find_map(|i| match i {
                Item::Note(n) => Some(n.span),
                _ => None,
            });
            cx.warn(
                sp.unwrap_or_default(),
                format!(
                    "Suara {} panjangnya {} ketuk, suara {} {} ketuk",
                    v.name,
                    fmt_beats(l),
                    first.name,
                    fmt_beats(len0)
                ),
            );
        }
    }
    // Birama yang panjangnya tidak sesuai ketuk (birama awal & akhir boleh tidak penuh).
    if let Some(bpb) = doc.meta.beats_per_bar() {
        for v in &doc.voices {
            let mut bars: Vec<(u32, Span)> = Vec::new();
            let mut acc = 0u32;
            let mut first_span = None;
            for it in &v.items {
                match it {
                    Item::Note(n) => {
                        acc += n.dur;
                        first_span.get_or_insert(n.span);
                    }
                    Item::Bar(_, sp) => {
                        if acc > 0 {
                            bars.push((acc, first_span.unwrap_or(*sp)));
                        }
                        acc = 0;
                        first_span = None;
                    }
                    _ => {}
                }
            }
            if acc > 0 {
                bars.push((acc, first_span.unwrap_or_default()));
            }
            let want = bpb * BEAT;
            let n = bars.len();
            for (k, (len, sp)) in bars.iter().enumerate() {
                let edge = k == 0 || k + 1 == n;
                if *len > want || (*len < want && !edge) {
                    cx.warn(
                        *sp,
                        format!("Birama ini {} ketuk, seharusnya {}", fmt_beats(*len), bpb),
                    );
                }
            }
        }
    }
    // Lirik lebih banyak dari not.
    let slots = doc.lyric_slots().len();
    for v in &doc.verses {
        if v.syllables.len() > slots {
            cx.warn(
                Span::default(),
                format!(
                    "Bait {}: {} suku kata, tetapi hanya ada {} not",
                    v.number,
                    v.syllables.len(),
                    slots
                ),
            );
        }
    }
}

pub fn fmt_beats(ticks: u32) -> String {
    if ticks % BEAT == 0 {
        (ticks / BEAT).to_string()
    } else {
        let v = ticks as f64 / BEAT as f64;
        let s = format!("{:.2}", v);
        s.trim_end_matches('0')
            .trim_end_matches('.')
            .replace('.', ",")
    }
}
