//! Formatter: mengubah model menjadi teks Nada yang rapi dan baku.
//!
//! Aturan:
//! - header urut baku, tanda = sejajar;
//! - satu ketuk = satu token; ketuk yang dibagi ditulis dengan [ ];
//! - empat birama per baris (atau mengikuti tanda \ bila ada);
//! - baris lirik dipotong mengikuti baris not suara pertama.

use crate::model::*;

/// Ubah teks menjadi teks baku. Bila ada kesalahan, teks dikembalikan apa adanya.
pub fn format_source(src: &str) -> Result<String, Vec<crate::parse::Diagnostic>> {
    let p = crate::parse::parse(src);
    if p.has_errors() {
        return Err(p.diagnostics);
    }
    Ok(format(&p.doc))
}

pub fn format(doc: &Doc) -> String {
    let mut doc = doc.clone();
    normalize(&mut doc);
    let mut out = String::new();

    for c in &doc.comments {
        out.push_str("// ");
        out.push_str(c);
        out.push('\n');
    }
    if !doc.comments.is_empty() {
        out.push('\n');
    }

    let metas: Vec<(&str, String)> = META_KEYS
        .iter()
        .filter_map(|k| doc.meta.get(k).map(|v| (*k, v)))
        .collect();
    let w = metas.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (k, v) in &metas {
        out.push_str(k);
        for _ in k.len()..w {
            out.push(' ');
        }
        out.push_str(" = ");
        out.push_str(&meta_value(k, v));
        out.push('\n');
    }

    let mut first_lines: Vec<usize> = Vec::new();
    for (vi, v) in doc.voices.iter().enumerate() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("suara ");
        out.push_str(&v.name);
        out.push_str(" {\n");
        let (lines, slots) = voice_lines(v);
        for l in &lines {
            out.push_str("  ");
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("}\n");
        if vi == 0 {
            first_lines = slots;
        }
    }

    for verse in &doc.verses {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("bait {} {{\n", verse.number));
        for l in lyric_lines(&verse.syllables, &first_lines) {
            out.push_str("  ");
            out.push_str(&l);
            out.push('\n');
        }
        out.push_str("}\n");
    }

    for t in &doc.texts {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("teks {} {{\n", t.number));
        for l in &t.lines {
            if l.is_empty() {
                out.push('\n');
            } else {
                out.push_str("  ");
                out.push_str(l);
                out.push('\n');
            }
        }
        out.push_str("}\n");
    }
    out
}

fn meta_value(key: &str, v: &str) -> String {
    let bare = matches!(key, "nada" | "ketuk" | "bpm")
        && !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '#');
    if bare {
        return v.to_string();
    }
    let mut s = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

// ---------- Normalisasi ketuk ----------

/// Pastikan setiap not berada di dalam satu ketuk dan bisa ditulis dengan [ ].
/// Not yang melewati batas dipecah menjadi not + titik. Ketuk terakhir yang belum
/// penuh dilengkapi dengan titik.
pub fn normalize(doc: &mut Doc) {
    for v in &mut doc.voices {
        normalize_voice(v);
    }
}

pub fn normalize_voice(v: &mut Voice) {
    // Buang not tanpa panjang.
    v.items
        .retain(|i| !matches!(i, Item::Note(n) if n.dur == 0));

    let mut out: Vec<Item> = Vec::with_capacity(v.items.len());
    let mut t: u32 = 0;
    for item in v.items.drain(..) {
        match item {
            Item::Note(n) => {
                for piece in split_note(n, t) {
                    t += piece.dur;
                    out.push(Item::Note(piece));
                }
            }
            other => out.push(other),
        }
    }
    let rem = t % BEAT;
    if rem != 0 {
        let pad = Note::new(Pitch::Hold, BEAT - rem);
        // Letakkan sebelum garis birama/pindah baris penutup.
        let pos = out
            .iter()
            .rposition(|i| matches!(i, Item::Note(_)))
            .map(|p| p + 1)
            .unwrap_or(out.len());
        out.insert(pos, Item::Note(pad));
    }
    v.items = out;
}

/// Pecah satu not yang dimulai pada `t` agar tidak melewati batas pembagian.
fn split_note(n: Note, t: u32) -> Vec<Note> {
    let mut pieces = Vec::new();
    let mut cur = n;
    let mut start = t;
    loop {
        let cut = cut_point(start, cur.dur);
        match cut {
            Some(c) if c > start && c < start + cur.dur => {
                let first_len = c - start;
                let mut head = cur.clone();
                head.dur = first_len;
                head.breath = false;
                head.slur_end = false;
                let mut tail = Note::new(Pitch::Hold, cur.dur - first_len);
                tail.breath = cur.breath;
                tail.fermata = false;
                if !cur.is_tone() {
                    tail.pitch = cur.pitch;
                    if matches!(cur.pitch, Pitch::Rest) {
                        tail.pitch = Pitch::Rest;
                    }
                }
                // Akhir legato tetap pada not bunyi (kepala).
                head.slur_end = cur.slur_end;
                pieces.push(head);
                start = c;
                cur = tail;
            }
            _ => {
                pieces.push(cur);
                break;
            }
        }
    }
    pieces
}

/// Titik potong pertama yang dilewati not [start, start+dur), atau None bila sudah rapi.
fn cut_point(start: u32, dur: u32) -> Option<u32> {
    let end = start + dur;
    let beat_end = (start / BEAT + 1) * BEAT;
    if end > beat_end {
        return Some(beat_end);
    }
    // Di dalam satu ketuk: periksa pembagian rekursif.
    let mut ws = start / BEAT * BEAT;
    let mut wl = BEAT;
    loop {
        if start == ws && dur == wl {
            return None;
        }
        let d = division(wl, start - ws, dur);
        let part = wl / d;
        if part == 0 || wl % d != 0 {
            return None;
        }
        let k = (start - ws) / part;
        let ps = ws + k * part;
        let pe = ps + part;
        if end > pe {
            return Some(pe);
        }
        if part <= BEAT / 16 {
            return None;
        }
        ws = ps;
        wl = part;
    }
}

/// Bagi dua, kecuali posisi/panjang hanya pas dengan pembagian tiga (triol).
fn division(wl: u32, off: u32, dur: u32) -> u32 {
    let half = wl / 2;
    if wl % 2 == 0 && off % half_unit(half) == 0 && dur % half_unit(half) == 0 {
        return 2;
    }
    if wl % 3 == 0 {
        let third = wl / 3;
        if off % third_unit(third) == 0 && dur % third_unit(third) == 0 {
            return 3;
        }
    }
    2
}

fn half_unit(half: u32) -> u32 {
    // Satuan terkecil biner di bawah setengah jendela.
    let mut u = half;
    while u % 2 == 0 && u > BEAT / 16 {
        u /= 2;
    }
    u.max(1)
}

fn third_unit(third: u32) -> u32 {
    let mut u = third;
    while u % 2 == 0 && u > BEAT / 48 {
        u /= 2;
    }
    u.max(1)
}

// ---------- Baris not ----------

struct Tokened {
    start: u32,
    dur: u32,
    tok: String,
}

/// Mengembalikan baris-baris kode suara dan jumlah slot lirik di tiap baris.
fn voice_lines(v: &Voice) -> (Vec<String>, Vec<usize>) {
    let mut lines = Vec::new();
    let mut slots_per_line = Vec::new();
    let has_breaks = v.items.iter().any(|i| matches!(i, Item::Break(_)));

    let mut line = String::new();
    let mut line_slots = 0usize;
    let mut bars_in_line = 0;
    let mut window: Vec<Tokened> = Vec::new();
    let mut t: u32 = 0;
    let mut in_slur = false;

    let flush_window = |window: &mut Vec<Tokened>, line: &mut String| {
        if window.is_empty() {
            return;
        }
        let ws = window[0].start / BEAT * BEAT;
        let s = render_window(window, ws, BEAT);
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&s);
        window.clear();
    };

    for item in &v.items {
        match item {
            Item::Note(n) => {
                if !window.is_empty() && t / BEAT != window[0].start / BEAT {
                    flush_window(&mut window, &mut line);
                }
                if n.is_tone() && (!in_slur || n.slur_start) {
                    line_slots += 1;
                }
                if n.is_tone() {
                    if n.slur_start {
                        in_slur = true;
                    }
                    if n.slur_end {
                        in_slur = false;
                    }
                }
                window.push(Tokened {
                    start: t,
                    dur: n.dur,
                    tok: note_token(n),
                });
                t += n.dur;
                if t % BEAT == 0 {
                    flush_window(&mut window, &mut line);
                }
            }
            Item::Bar(k, _) => {
                flush_window(&mut window, &mut line);
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(k.token());
                if *k != BarKind::RepeatStart {
                    bars_in_line += 1;
                }
                let wrap = !has_breaks && bars_in_line >= 4 && *k != BarKind::RepeatStart;
                if wrap {
                    lines.push(std::mem::take(&mut line));
                    slots_per_line.push(line_slots);
                    line_slots = 0;
                    bars_in_line = 0;
                }
            }
            Item::Break(_) => {
                flush_window(&mut window, &mut line);
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push('\\');
                lines.push(std::mem::take(&mut line));
                slots_per_line.push(line_slots);
                line_slots = 0;
                bars_in_line = 0;
            }
        }
    }
    flush_window(&mut window, &mut line);
    if !line.is_empty() {
        lines.push(line);
        slots_per_line.push(line_slots);
    }
    (lines, slots_per_line)
}

fn render_window(notes: &[Tokened], ws: u32, wl: u32) -> String {
    if notes.len() == 1 && notes[0].start == ws && notes[0].dur == wl {
        return notes[0].tok.clone();
    }
    // Semua sama panjang -> satu kurung datar, mis. [1 2 3 4].
    let k = notes.len() as u32;
    if k > 1
        && wl % k == 0
        && notes
            .iter()
            .enumerate()
            .all(|(i, n)| n.dur == wl / k && n.start == ws + i as u32 * (wl / k))
    {
        let inner: Vec<&str> = notes.iter().map(|n| n.tok.as_str()).collect();
        return format!("[{}]", inner.join(" "));
    }
    let off = notes.iter().map(|n| n.start - ws).fold(0, gcd);
    let durg = notes.iter().map(|n| n.dur).fold(0, gcd);
    let d = division(wl, off, durg);
    let part = wl / d;
    let mut parts = Vec::new();
    for p in 0..d {
        let ps = ws + p * part;
        let sub: Vec<Tokened> = notes
            .iter()
            .filter(|n| n.start >= ps && n.start < ps + part)
            .map(|n| Tokened {
                start: n.start,
                dur: n.dur,
                tok: n.tok.clone(),
            })
            .collect();
        if sub.is_empty() || part == 0 {
            // Tidak semestinya terjadi setelah normalisasi; tulis apa adanya.
            let inner: Vec<&str> = notes.iter().map(|n| n.tok.as_str()).collect();
            return format!("[{}]", inner.join(" "));
        }
        parts.push(render_window(&sub, ps, part));
    }
    format!("[{}]", parts.join(" "))
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub fn note_token(n: &Note) -> String {
    let mut s = String::new();
    if let Some(l) = &n.label {
        if l.chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '!')
            && !l.is_empty()
        {
            s.push('@');
            s.push_str(l);
        } else {
            s.push_str("@\"");
            s.push_str(&l.replace('"', "'"));
            s.push('"');
        }
        s.push(' ');
    }
    if n.slur_start {
        s.push('(');
    }
    match n.pitch {
        Pitch::Hold => s.push('.'),
        Pitch::Rest => s.push('0'),
        Pitch::Tone {
            degree,
            octave,
            accidental,
        } => {
            s.push((b'0' + degree) as char);
            let mark = if octave > 0 { '\'' } else { ',' };
            for _ in 0..octave.unsigned_abs() {
                s.push(mark);
            }
            match accidental {
                1 => s.push('#'),
                -1 => s.push('b'),
                _ => {}
            }
        }
    }
    if n.fermata {
        s.push('^');
    }
    if n.slur_end {
        s.push(')');
    }
    if n.breath {
        s.push_str(" ;");
    }
    s
}

// ---------- Lirik ----------

fn lyric_lines(syl: &[Option<Syllable>], per_line: &[usize]) -> Vec<String> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    for &n in per_line {
        if i >= syl.len() {
            break;
        }
        let end = (i + n).min(syl.len());
        if end > i {
            ranges.push((i, end));
        }
        i = end;
    }
    // Suku kata berlebih ikut baris terakhir.
    if i < syl.len() {
        match ranges.last_mut() {
            Some(r) => r.1 = syl.len(),
            None => ranges.push((i, syl.len())),
        }
    }
    ranges
        .into_iter()
        .map(|(a, b)| join_syllables(&syl[a..b]))
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn join_syllables(syl: &[Option<Syllable>]) -> String {
    let mut s = String::new();
    let mut glue = false;
    for x in syl {
        if !s.is_empty() && !glue {
            s.push(' ');
        }
        match x {
            None => {
                s.push('_');
                glue = false;
            }
            Some(y) => {
                s.push_str(&y.text);
                if y.hyphen {
                    s.push('-');
                }
                glue = y.hyphen;
            }
        }
    }
    s
}
