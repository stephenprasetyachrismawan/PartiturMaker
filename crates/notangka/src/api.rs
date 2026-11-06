//! API JSON tunggal. Dipakai oleh WASM (editor) dan aplikasi desktop.
//!
//! Masukan: `{"fn": "...", "src": "...", ...}`, keluaran: JSON.

use crate::edit::{apply, Op};
use crate::layout::{layout, Options};
use crate::model::*;
use crate::parse::{parse, Severity};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct Req {
    #[serde(rename = "fn")]
    func: String,
    #[serde(default)]
    src: String,
    #[serde(default)]
    opts: Option<Value>,
    #[serde(default)]
    op: Option<Value>,
}

pub fn handle(input: &str) -> String {
    let req: Req = match serde_json::from_str(input) {
        Ok(r) => r,
        Err(e) => return json!({ "error": format!("Permintaan tidak sah: {}", e) }).to_string(),
    };
    let out = match req.func.as_str() {
        "render" => render(&req),
        "format" => format(&req),
        "apply" => apply_op(&req),
        "pdf" => pdf(&req),
        "timeline" => timeline(&req.src),
        other => json!({ "error": format!("Fungsi \"{}\" tidak ada", other) }),
    };
    out.to_string()
}

fn opts_of(req: &Req) -> Options {
    req.opts
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn structure(doc: &Doc) -> Value {
    let slots = doc.lyric_slots();
    let voices: Vec<Value> = doc
        .voices
        .iter()
        .enumerate()
        .map(|(vi, v)| {
            let mut t = 0u32;
            let items: Vec<Value> = v
                .items
                .iter()
                .enumerate()
                .map(|(ii, it)| match it {
                    Item::Note(n) => {
                        let slot = if vi == 0 { slots.iter().position(|&x| x == ii) } else { None };
                        let o = json!({
                            "slot": slot,
                            "k": match n.pitch { Pitch::Tone { .. } => "n", Pitch::Rest => "r", Pitch::Hold => "h" },
                            "t": t, "dur": n.dur,
                            "slur": n.slur_start || n.slur_end, "breath": n.breath, "fermata": n.fermata,
                            "label": n.label,
                            "acc": if let Pitch::Tone { accidental, .. } = n.pitch { accidental } else { 0 },
                            "oct": if let Pitch::Tone { octave, .. } = n.pitch { octave } else { 0 },
                        });
                        t += n.dur;
                        o
                    }
                    Item::Bar(k, _) => json!({ "k": "b", "t": t, "bar": k.token() }),
                    Item::Break(_) => json!({ "k": "br", "t": t }),
                })
                .collect();
            json!({ "name": v.name, "items": items })
        })
        .collect();
    let meta: serde_json::Map<String, Value> = META_KEYS
        .iter()
        .map(|k| (k.to_string(), json!(doc.meta.get(k))))
        .collect();
    json!({
        "voices": voices,
        "verses": doc.verses.iter().map(|v| json!({ "number": v.number, "syllables": v.syllables.iter().map(|s| s.as_ref().map(|x| if x.hyphen { format!("{}-", x.text) } else { x.text.clone() })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "texts": doc.texts.iter().map(|t| json!({ "number": t.number, "text": t.lines.join("\n") })).collect::<Vec<_>>(),
        "slots": doc.lyric_slots().len(),
        "meta": meta,
    })
}

fn render(req: &Req) -> Value {
    let p = parse(&req.src);
    let opts = opts_of(req);
    let pages = layout(&p.doc, &opts);
    let svgs: Vec<String> = pages
        .iter()
        .map(|pg| crate::svg::render(pg, opts.editing))
        .collect();
    let mut norm = p.doc.clone();
    crate::format::normalize(&mut norm);
    json!({
        "pages": svgs,
        "size": pages.first().map(|p| [p.width, p.height]),
        "diagnostics": p.diagnostics,
        "doc": structure(&norm),
    })
}

fn format(req: &Req) -> Value {
    match crate::format::format_source(&req.src) {
        Ok(s) => json!({ "src": s }),
        Err(d) => json!({ "diagnostics": d }),
    }
}

fn apply_op(req: &Req) -> Value {
    let p = parse(&req.src);
    if p.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return json!({ "error": "Ada kesalahan di kode. Perbaiki dulu di panel kode, lalu sunting lagi di partitur." });
    }
    let Some(opv) = req.op.clone() else {
        return json!({ "error": "Operasi kosong" });
    };
    let op: Op = match serde_json::from_value(opv) {
        Ok(o) => o,
        Err(e) => return json!({ "error": format!("Operasi tidak dikenal: {}", e) }),
    };
    let mut doc = p.doc;
    crate::format::normalize(&mut doc);
    match apply(&mut doc, &op) {
        Ok(caret) => json!({ "src": crate::format::format(&doc), "caret": caret }),
        Err(e) => json!({ "error": e }),
    }
}

fn pdf(req: &Req) -> Value {
    let p = parse(&req.src);
    let mut opts = opts_of(req);
    opts.editing = false;
    let pages = layout(&p.doc, &opts);
    let title = p
        .doc
        .meta
        .title
        .clone()
        .unwrap_or_else(|| "Partitur".into());
    let bytes = crate::pdf::render(&pages, &title);
    json!({ "pdf": base64(&bytes) })
}

const SEMI: [i32; 8] = [0, 0, 2, 4, 5, 7, 9, 11];

/// Daftar nada untuk diputar. Tenor dan bas ditulis satu oktaf di atas bunyinya.
pub fn timeline(src: &str) -> Value {
    let p = parse(src);
    let mut doc = p.doc;
    crate::format::normalize(&mut doc);
    let tonic = 60 + doc.meta.key.as_deref().map(key_pitch_class).unwrap_or(0) as i32;
    let bpm = doc.meta.bpm.unwrap_or(88).clamp(30, 240);

    // Fermata menahan semua suara satu ketuk.
    let mut holds: Vec<u32> = Vec::new();
    for v in &doc.voices {
        let mut t = 0;
        for n in v.notes() {
            t += n.dur;
            if n.fermata && !holds.contains(&t) {
                holds.push(t);
            }
        }
    }
    let warp = |t: u32| t + holds.iter().filter(|&&h| h < t).count() as u32 * BEAT;

    let mut notes = Vec::new();
    for (vi, v) in doc.voices.iter().enumerate() {
        let shift = if v.name == "T" || v.name == "B" {
            -12
        } else {
            0
        };
        let mut t = 0u32;
        let mut last: Option<(i32, u32, u32, usize, bool)> = None; // midi, start, dur, index, slur terbuka
        let mut out: Vec<(i32, u32, u32, usize)> = Vec::new();
        for (ii, it) in v.items.iter().enumerate() {
            let Item::Note(n) = it else { continue };
            match n.pitch {
                Pitch::Tone {
                    degree,
                    octave,
                    accidental,
                } => {
                    let midi = tonic
                        + SEMI[degree as usize]
                        + accidental as i32
                        + 12 * octave as i32
                        + shift;
                    let extra = if n.fermata { BEAT } else { 0 };
                    match &mut last {
                        Some(l) if l.4 && l.0 == midi && l.1 + l.2 == warp(t) => {
                            l.2 += n.dur + extra;
                            l.4 = !n.slur_end;
                        }
                        _ => {
                            if let Some(l) = last.take() {
                                out.push((l.0, l.1, l.2, l.3));
                            }
                            last = Some((midi, warp(t), n.dur + extra, ii, n.slur_start));
                        }
                    }
                    if n.slur_start {
                        if let Some(l) = &mut last {
                            l.4 = true;
                        }
                    }
                }
                Pitch::Hold => {
                    if let Some(l) = &mut last {
                        l.2 += n.dur + if n.fermata { BEAT } else { 0 };
                    }
                }
                Pitch::Rest => {
                    if let Some(l) = last.take() {
                        out.push((l.0, l.1, l.2, l.3));
                    }
                }
            }
            t += n.dur;
        }
        if let Some(l) = last.take() {
            out.push((l.0, l.1, l.2, l.3));
        }
        for (midi, start, dur, index) in out {
            notes.push(json!({ "voice": vi, "index": index, "midi": midi, "start": start as f32 / BEAT as f32, "dur": dur as f32 / BEAT as f32 }));
        }
    }
    json!({ "bpm": bpm, "notes": notes })
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        s.push(T[(v >> 18) as usize & 63] as char);
        s.push(T[(v >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 {
            T[(v >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            T[v as usize & 63] as char
        } else {
            '='
        });
    }
    s
}
