//! Operasi sunting dari editor visual. Setiap operasi mengubah model, lalu
//! model ditulis ulang menjadi kode Nada oleh formatter.

use crate::format::normalize_voice;
use crate::model::*;
use crate::parse::parse_lyrics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    SetMeta {
        key: String,
        value: Option<String>,
    },
    /// Tulis not di posisi `at` (sisip pada batas ketuk, timpa di tengah ketuk).
    Insert {
        voice: usize,
        at: usize,
        digit: char,
        #[serde(default)]
        octave: i8,
        #[serde(default)]
        accidental: i8,
        dur: u32,
    },
    /// Ganti nada not yang dipilih, panjang tetap.
    SetPitch {
        voice: usize,
        at: usize,
        digit: char,
        #[serde(default)]
        octave: Option<i8>,
        #[serde(default)]
        accidental: Option<i8>,
    },
    Octave {
        voice: usize,
        at: usize,
        delta: i8,
    },
    Accidental {
        voice: usize,
        at: usize,
        value: i8,
    },
    SetDuration {
        voice: usize,
        at: usize,
        dur: u32,
    },
    InsertBar {
        voice: usize,
        at: usize,
        kind: BarKindName,
    },
    SetBar {
        voice: usize,
        at: usize,
        kind: BarKindName,
    },
    Delete {
        voice: usize,
        at: usize,
    },
    ToggleSlur {
        voice: usize,
        at: usize,
    },
    ToggleFermata {
        voice: usize,
        at: usize,
    },
    ToggleBreath {
        voice: usize,
        at: usize,
    },
    SetLabel {
        voice: usize,
        at: usize,
        text: Option<String>,
    },
    ToggleBreak {
        voice: usize,
        at: usize,
    },
    SetSyllable {
        verse: u32,
        slot: usize,
        text: String,
    },
    AddVerse {},
    RemoveVerse {
        verse: u32,
    },
    AddVoice {
        name: String,
    },
    RemoveVoice {
        voice: usize,
    },
    SetTextVerse {
        number: u32,
        text: String,
    },
    AddTextVerse {},
    RemoveTextVerse {
        number: u32,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BarKindName {
    Single,
    Double,
    Final,
    RepeatStart,
    RepeatEnd,
    RepeatBoth,
}

impl From<BarKindName> for BarKind {
    fn from(k: BarKindName) -> Self {
        match k {
            BarKindName::Single => BarKind::Single,
            BarKindName::Double => BarKind::Double,
            BarKindName::Final => BarKind::Final,
            BarKindName::RepeatStart => BarKind::RepeatStart,
            BarKindName::RepeatEnd => BarKind::RepeatEnd,
            BarKindName::RepeatBoth => BarKind::RepeatBoth,
        }
    }
}

/// Posisi kursor sesudah operasi.
#[derive(Debug, Clone, Copy, Serialize, Default)]
pub struct Caret {
    pub voice: usize,
    pub index: usize,
}

pub fn apply(doc: &mut Doc, op: &Op) -> Result<Option<Caret>, String> {
    let caret = match op {
        Op::SetMeta { key, value } => {
            if !doc.meta.set(key, value.clone()) {
                return Err(format!("Kunci \"{}\" tidak dikenal", key));
            }
            None
        }
        Op::Insert {
            voice,
            at,
            digit,
            octave,
            accidental,
            dur,
        } => {
            let pitch = pitch_from(*digit, *octave, *accidental)?;
            let v = voice_mut(doc, *voice)?;
            Some(insert(v, *voice, *at, pitch, *dur)?)
        }
        Op::SetPitch {
            voice,
            at,
            digit,
            octave,
            accidental,
        } => {
            let v = voice_mut(doc, *voice)?;
            let n = note_mut(v, *at)?;
            let (old_oct, old_acc) = match n.pitch {
                Pitch::Tone {
                    octave, accidental, ..
                } => (octave, accidental),
                _ => (0, 0),
            };
            // Angka baru: oktaf tetap, kres/mol dibuang kecuali diminta.
            let _ = old_acc;
            n.pitch = pitch_from(*digit, octave.unwrap_or(old_oct), accidental.unwrap_or(0))?;
            if !n.is_tone() {
                n.slur_start = false;
                n.slur_end = false;
            }
            Some(Caret {
                voice: *voice,
                index: next_note_index(v, *at),
            })
        }
        Op::Octave { voice, at, delta } => {
            let n = note_mut(voice_mut(doc, *voice)?, *at)?;
            if let Pitch::Tone { octave, .. } = &mut n.pitch {
                *octave = (*octave + delta).clamp(-3, 3);
            }
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::Accidental { voice, at, value } => {
            let n = note_mut(voice_mut(doc, *voice)?, *at)?;
            if let Pitch::Tone { accidental, .. } = &mut n.pitch {
                *accidental = if *accidental == *value {
                    0
                } else {
                    (*value).clamp(-1, 1)
                };
            }
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::SetDuration { voice, at, dur } => {
            let v = voice_mut(doc, *voice)?;
            set_duration(v, *at, *dur)?;
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::InsertBar { voice, at, kind } => {
            let v = voice_mut(doc, *voice)?;
            let at = snap_to_beat(v, *at);
            v.items
                .insert(at, Item::Bar((*kind).into(), Span::default()));
            Some(Caret {
                voice: *voice,
                index: at + 1,
            })
        }
        Op::SetBar { voice, at, kind } => {
            let v = voice_mut(doc, *voice)?;
            match v.items.get_mut(*at) {
                Some(Item::Bar(k, _)) => *k = (*kind).into(),
                _ => return Err("Yang dipilih bukan garis birama".into()),
            }
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::Delete { voice, at } => {
            let v = voice_mut(doc, *voice)?;
            delete(v, *at)?;
            Some(Caret {
                voice: *voice,
                index: (*at).min(v.items.len()),
            })
        }
        Op::ToggleSlur { voice, at } => {
            let v = voice_mut(doc, *voice)?;
            toggle_slur(v, *at)?;
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::ToggleFermata { voice, at } => {
            let n = note_mut(voice_mut(doc, *voice)?, *at)?;
            n.fermata = !n.fermata;
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::ToggleBreath { voice, at } => {
            let n = note_mut(voice_mut(doc, *voice)?, *at)?;
            n.breath = !n.breath;
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::SetLabel { voice, at, text } => {
            let n = note_mut(voice_mut(doc, *voice)?, *at)?;
            n.label = text
                .clone()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            Some(Caret {
                voice: *voice,
                index: *at,
            })
        }
        Op::ToggleBreak { voice, at } => {
            let v = voice_mut(doc, *voice)?;
            if *at > 0 && matches!(v.items.get(*at - 1), Some(Item::Break(_))) {
                v.items.remove(*at - 1);
                Some(Caret {
                    voice: *voice,
                    index: *at - 1,
                })
            } else {
                // Pindah baris sebaiknya sesudah garis birama.
                let mut pos = (*at).min(v.items.len());
                if matches!(v.items.get(pos), Some(Item::Bar(..))) {
                    pos += 1;
                }
                v.items.insert(pos, Item::Break(Span::default()));
                Some(Caret {
                    voice: *voice,
                    index: pos + 1,
                })
            }
        }
        Op::SetSyllable { verse, slot, text } => {
            let v = match doc.verses.iter_mut().find(|v| v.number == *verse) {
                Some(v) => v,
                None => {
                    doc.verses.push(Verse {
                        number: *verse,
                        syllables: Vec::new(),
                    });
                    doc.verses.sort_by_key(|v| v.number);
                    doc.verses.iter_mut().find(|v| v.number == *verse).unwrap()
                }
            };
            while v.syllables.len() <= *slot {
                v.syllables.push(None);
            }
            let mut parts = parse_lyrics(text);
            if parts.is_empty() {
                parts.push(None);
            }
            v.syllables.splice(*slot..*slot + 1, parts);
            while matches!(v.syllables.last(), Some(None)) {
                v.syllables.pop();
            }
            None
        }
        Op::AddVerse {} => {
            let n = doc
                .verses
                .iter()
                .map(|v| v.number)
                .chain(doc.texts.iter().map(|t| t.number))
                .max()
                .unwrap_or(0)
                + 1;
            doc.verses.push(Verse {
                number: n,
                syllables: Vec::new(),
            });
            doc.verses.sort_by_key(|v| v.number);
            None
        }
        Op::RemoveVerse { verse } => {
            doc.verses.retain(|v| v.number != *verse);
            None
        }
        Op::AddVoice { name } => {
            let name =
                voice_name(name).ok_or_else(|| format!("Suara \"{}\" tidak dikenal", name))?;
            if doc.voices.iter().any(|v| v.name == name) {
                return Err(format!("Suara {} sudah ada", name));
            }
            // Salin irama suara pertama sebagai tanda diam, agar langsung sejajar.
            let items = match doc.voices.first() {
                Some(first) => first
                    .items
                    .iter()
                    .map(|it| match it {
                        Item::Note(n) => {
                            let mut m = Note::new(
                                if matches!(n.pitch, Pitch::Hold) {
                                    Pitch::Hold
                                } else {
                                    Pitch::Rest
                                },
                                n.dur,
                            );
                            m.fermata = n.fermata;
                            Item::Note(m)
                        }
                        other => other.clone(),
                    })
                    .collect(),
                None => Vec::new(),
            };
            doc.voices.push(Voice {
                name: name.to_string(),
                items,
            });
            doc.voices
                .sort_by_key(|v| VOICE_ORDER.iter().position(|n| *n == v.name).unwrap_or(9));
            let vi = doc.voices.iter().position(|v| v.name == name).unwrap_or(0);
            Some(Caret {
                voice: vi,
                index: 0,
            })
        }
        Op::RemoveVoice { voice } => {
            if *voice >= doc.voices.len() {
                return Err("Suara tidak ada".into());
            }
            doc.voices.remove(*voice);
            Some(Caret { voice: 0, index: 0 })
        }
        Op::SetTextVerse { number, text } => {
            let lines: Vec<String> = text.lines().map(|l| l.trim().to_string()).collect();
            match doc.texts.iter_mut().find(|t| t.number == *number) {
                Some(t) => t.lines = lines,
                None => {
                    doc.texts.push(TextVerse {
                        number: *number,
                        lines,
                    });
                    doc.texts.sort_by_key(|t| t.number);
                }
            }
            None
        }
        Op::AddTextVerse {} => {
            let n = doc
                .verses
                .iter()
                .map(|v| v.number)
                .chain(doc.texts.iter().map(|t| t.number))
                .max()
                .unwrap_or(0)
                + 1;
            doc.texts.push(TextVerse {
                number: n,
                lines: vec![String::new()],
            });
            None
        }
        Op::RemoveTextVerse { number } => {
            doc.texts.retain(|t| t.number != *number);
            None
        }
    };
    for v in &mut doc.voices {
        normalize_voice(v);
    }
    Ok(caret)
}

fn pitch_from(digit: char, octave: i8, accidental: i8) -> Result<Pitch, String> {
    Ok(match digit {
        '.' => Pitch::Hold,
        '0' => Pitch::Rest,
        '1'..='7' => Pitch::Tone {
            degree: digit as u8 - b'0',
            octave: octave.clamp(-3, 3),
            accidental: accidental.clamp(-1, 1),
        },
        _ => return Err(format!("\"{}\" bukan not angka", digit)),
    })
}

fn voice_mut(doc: &mut Doc, i: usize) -> Result<&mut Voice, String> {
    if doc.voices.is_empty() && i == 0 {
        doc.voices.push(Voice {
            name: "S".into(),
            items: Vec::new(),
        });
    }
    doc.voices
        .get_mut(i)
        .ok_or_else(|| "Suara tidak ada".to_string())
}

fn note_mut(v: &mut Voice, at: usize) -> Result<&mut Note, String> {
    match v.items.get_mut(at) {
        Some(Item::Note(n)) => Ok(n),
        _ => Err("Pilih sebuah not dulu".into()),
    }
}

/// Waktu mulai item ke-`at`.
fn time_at(v: &Voice, at: usize) -> u32 {
    v.items
        .iter()
        .take(at)
        .map(|i| if let Item::Note(n) = i { n.dur } else { 0 })
        .sum()
}

fn next_note_index(v: &Voice, at: usize) -> usize {
    let mut i = at + 1;
    while i < v.items.len() && !matches!(v.items[i], Item::Note(_)) {
        i += 1;
    }
    i.min(v.items.len())
}

fn snap_to_beat(v: &Voice, at: usize) -> usize {
    let mut at = at.min(v.items.len());
    while at < v.items.len() && time_at(v, at) % BEAT != 0 {
        at += 1;
    }
    at
}

fn insert(v: &mut Voice, vi: usize, at: usize, pitch: Pitch, dur: u32) -> Result<Caret, String> {
    let dur = dur.clamp(BEAT / 16, BEAT);
    let at = at.min(v.items.len());
    let t = time_at(v, at);
    let in_beat = t % BEAT;
    if in_beat == 0 {
        // Sisip pada batas ketuk: not + titik pelengkap bila kurang dari satu ketuk.
        v.items.insert(at, Item::Note(Note::new(pitch, dur)));
        if dur < BEAT {
            v.items
                .insert(at + 1, Item::Note(Note::new(Pitch::Hold, BEAT - dur)));
        }
        return Ok(Caret {
            voice: vi,
            index: at + 1,
        });
    }
    // Di tengah ketuk: timpa sepanjang `dur`, tidak melewati akhir ketuk.
    let beat_end = t - in_beat + BEAT;
    let dur = dur.min(beat_end - t);
    let mut remaining = dur;
    let mut i = at;
    while remaining > 0 && i < v.items.len() {
        match &mut v.items[i] {
            Item::Note(n) => {
                if n.dur <= remaining {
                    remaining -= n.dur;
                    v.items.remove(i);
                } else {
                    n.dur -= remaining;
                    remaining = 0;
                }
            }
            _ => i += 1,
        }
    }
    v.items.insert(at, Item::Note(Note::new(pitch, dur)));
    Ok(Caret {
        voice: vi,
        index: at + 1,
    })
}

fn set_duration(v: &mut Voice, at: usize, dur: u32) -> Result<(), String> {
    let t = time_at(v, at);
    let old = note_mut(v, at)?.dur;
    let beat_end = t - t % BEAT + BEAT;
    let dur = dur.clamp(BEAT / 16, beat_end - t);
    if dur == old {
        return Ok(());
    }
    if dur < old {
        note_mut(v, at)?.dur = dur;
        v.items
            .insert(at + 1, Item::Note(Note::new(Pitch::Hold, old - dur)));
    } else {
        note_mut(v, at)?.dur = dur;
        let mut extra = dur - old;
        let i = at + 1;
        while extra > 0 && i < v.items.len() {
            match &mut v.items[i] {
                Item::Note(n) => {
                    if n.dur <= extra {
                        extra -= n.dur;
                        v.items.remove(i);
                    } else {
                        n.dur -= extra;
                        extra = 0;
                    }
                }
                _ => break,
            }
        }
        if extra > 0 {
            note_mut(v, at)?.dur -= extra;
        }
    }
    Ok(())
}

fn delete(v: &mut Voice, at: usize) -> Result<(), String> {
    match v.items.get(at) {
        None => Err("Tidak ada yang bisa dihapus".into()),
        Some(Item::Bar(..)) | Some(Item::Break(_)) => {
            v.items.remove(at);
            Ok(())
        }
        Some(Item::Note(n)) => {
            let t = time_at(v, at);
            if n.dur == BEAT && t % BEAT == 0 {
                v.items.remove(at);
                return Ok(());
            }
            // Bagian ketuk: panjangnya diberikan ke not tetangga di ketuk yang sama.
            let d = n.dur;
            let same_beat_prev =
                at > 0 && t % BEAT != 0 && matches!(v.items.get(at - 1), Some(Item::Note(_)));
            v.items.remove(at);
            if same_beat_prev {
                if let Some(Item::Note(p)) = v.items.get_mut(at - 1) {
                    p.dur += d;
                }
            } else if let Some(Item::Note(nx)) = v.items.get_mut(at) {
                nx.dur += d;
            }
            Ok(())
        }
    }
}

fn toggle_slur(v: &mut Voice, at: usize) -> Result<(), String> {
    let n = note_mut(v, at)?;
    if !n.is_tone() {
        return Err("Legato hanya untuk not 1 sampai 7".into());
    }
    if n.slur_start {
        n.slur_start = false;
        for it in v.items.iter_mut().skip(at + 1) {
            if let Item::Note(m) = it {
                if m.slur_end {
                    m.slur_end = false;
                    break;
                }
            }
        }
        return Ok(());
    }
    if n.slur_end {
        n.slur_end = false;
        for it in v.items[..at].iter_mut().rev() {
            if let Item::Note(m) = it {
                if m.slur_start {
                    m.slur_start = false;
                    break;
                }
            }
        }
        return Ok(());
    }
    // Sambung ke not bunyi berikutnya.
    let next = v
        .items
        .iter()
        .enumerate()
        .skip(at + 1)
        .find(|(_, it)| matches!(it, Item::Note(m) if m.is_tone()))
        .map(|(i, _)| i);
    let Some(j) = next else {
        return Err("Tidak ada not sesudahnya untuk disambung".into());
    };
    if let Item::Note(m) = &mut v.items[j] {
        if m.slur_start {
            // Gabungkan dengan legato berikutnya.
            m.slur_start = false;
        } else {
            m.slur_end = true;
        }
    }
    note_mut(v, at)?.slur_start = true;
    Ok(())
}
