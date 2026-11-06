//! Model lagu. Semua tampilan dan semua suntingan bekerja pada struktur ini.

use serde::{Deserialize, Serialize};

/// Satu ketuk = 960 tik. Habis dibagi 2, 3, 4, 8, 16, 6, 12.
pub const BEAT: u32 = 960;

/// Rentang byte di teks sumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pitch {
    /// Not 1..7 dengan oktaf (+ atas, - bawah) dan aksidental (+1 kres, -1 mol).
    Tone {
        degree: u8,
        octave: i8,
        accidental: i8,
    },
    /// 0, tanda diam.
    Rest,
    /// Titik: memperpanjang bunyi sebelumnya.
    Hold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub pitch: Pitch,
    /// Panjang dalam tik.
    pub dur: u32,
    pub slur_start: bool,
    pub slur_end: bool,
    pub fermata: bool,
    /// Tanda napas sesudah not ini.
    pub breath: bool,
    /// Teks kecil di atas not, mis. "Refrein" atau "Fine".
    pub label: Option<String>,
    #[serde(skip)]
    pub span: Span,
}

impl Note {
    pub fn new(pitch: Pitch, dur: u32) -> Self {
        Note {
            pitch,
            dur,
            slur_start: false,
            slur_end: false,
            fermata: false,
            breath: false,
            label: None,
            span: Span::default(),
        }
    }

    pub fn is_tone(&self) -> bool {
        matches!(self.pitch, Pitch::Tone { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarKind {
    Single,
    Double,
    Final,
    RepeatStart,
    RepeatEnd,
    RepeatBoth,
}

impl BarKind {
    pub fn token(self) -> &'static str {
        match self {
            BarKind::Single => "|",
            BarKind::Double => "||",
            BarKind::Final => "|.",
            BarKind::RepeatStart => "|:",
            BarKind::RepeatEnd => ":|",
            BarKind::RepeatBoth => ":|:",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Note(Note),
    Bar(BarKind, #[serde(skip)] Span),
    /// Paksa pindah baris di partitur.
    Break(#[serde(skip)] Span),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Voice {
    /// S, A, T atau B.
    pub name: String,
    pub items: Vec<Item>,
}

impl Voice {
    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.items.iter().filter_map(|i| match i {
            Item::Note(n) => Some(n),
            _ => None,
        })
    }

    pub fn length(&self) -> u32 {
        self.notes().map(|n| n.dur).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Syllable {
    pub text: String,
    /// Disambung ke suku kata berikutnya dengan tanda hubung.
    pub hyphen: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verse {
    pub number: u32,
    /// `None` = lewati satu not.
    pub syllables: Vec<Option<Syllable>>,
}

/// Bait yang dicetak sebagai teks biasa di bawah partitur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextVerse {
    pub number: u32,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Meta {
    pub number: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    /// Nada dasar, mis. "F", "Bes", "Fis".
    pub key: Option<String>,
    /// Isi "ketuk": "4" atau "6/8".
    pub beats: Option<String>,
    pub tempo: Option<String>,
    pub bpm: Option<u32>,
    pub lyricist: Option<String>,
    pub composer: Option<String>,
    pub translator: Option<String>,
    pub arranger: Option<String>,
    pub note: Option<String>,
}

/// Kunci header beserta urutan bakunya.
pub const META_KEYS: &[&str] = &[
    "nomor",
    "judul",
    "ayat",
    "nada",
    "ketuk",
    "tempo",
    "bpm",
    "syair",
    "lagu",
    "terjemahan",
    "aransemen",
    "catatan",
];

impl Meta {
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "nomor" => self.number.clone(),
            "judul" => self.title.clone(),
            "ayat" => self.subtitle.clone(),
            "nada" => self.key.clone(),
            "ketuk" => self.beats.clone(),
            "tempo" => self.tempo.clone(),
            "bpm" => self.bpm.map(|b| b.to_string()),
            "syair" => self.lyricist.clone(),
            "lagu" => self.composer.clone(),
            "terjemahan" => self.translator.clone(),
            "aransemen" => self.arranger.clone(),
            "catatan" => self.note.clone(),
            _ => None,
        }
    }

    /// Mengisi satu kunci. Mengembalikan `false` bila kunci tidak dikenal.
    pub fn set(&mut self, key: &str, value: Option<String>) -> bool {
        let v = value
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        match key {
            "nomor" => self.number = v,
            "judul" => self.title = v,
            "ayat" => self.subtitle = v,
            "nada" => self.key = v,
            "ketuk" => self.beats = v,
            "tempo" => self.tempo = v,
            "bpm" => self.bpm = v.and_then(|s| s.parse().ok()),
            "syair" => self.lyricist = v,
            "lagu" => self.composer = v,
            "terjemahan" => self.translator = v,
            "aransemen" => self.arranger = v,
            "catatan" => self.note = v,
            _ => return false,
        }
        true
    }

    /// Jumlah ketuk per birama, dari "4" atau "6/8".
    pub fn beats_per_bar(&self) -> Option<u32> {
        let b = self.beats.as_deref()?;
        b.split('/').next()?.trim().parse().ok()
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Doc {
    /// Komentar di awal berkas (tanpa "//").
    pub comments: Vec<String>,
    pub meta: Meta,
    pub voices: Vec<Voice>,
    pub verses: Vec<Verse>,
    pub texts: Vec<TextVerse>,
}

pub const VOICE_ORDER: &[&str] = &["S", "A", "T", "B"];

/// Nama suara baku dari tulisan pengguna.
pub fn voice_name(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().as_str() {
        "s" | "sopran" | "soprano" | "melodi" | "m" => Some("S"),
        "a" | "alto" => Some("A"),
        "t" | "tenor" => Some("T"),
        "b" | "bas" | "bass" => Some("B"),
        _ => None,
    }
}

impl Doc {
    /// Not-not di suara pertama yang menerima suku kata (not bunyi, bukan lanjutan legato).
    pub fn lyric_slots(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(v) = self.voices.first() else {
            return out;
        };
        let mut in_slur = false;
        for (i, item) in v.items.iter().enumerate() {
            if let Item::Note(n) = item {
                if n.is_tone() {
                    if !in_slur || n.slur_start {
                        out.push(i);
                    }
                    if n.slur_start {
                        in_slur = true;
                    }
                    if n.slur_end {
                        in_slur = false;
                    }
                }
            }
        }
        out
    }
}

/// Kelas nada 0..11 dari nama nada dasar (C, Cis, Des, D, Es, F, Fis, G, As, A, Bes, B).
pub fn key_pitch_class(key: &str) -> u8 {
    let s = key.trim();
    let s = s.rsplit('=').next().unwrap_or(s).trim();
    let mut chars = s.chars();
    let Some(first) = chars.next() else { return 0 };
    let base: i32 = match first.to_ascii_lowercase() {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return 0,
    };
    let rest: String = chars.collect::<String>().to_ascii_lowercase();
    let rest = rest.trim();
    let acc = if rest.starts_with("is") || rest.starts_with('#') {
        1
    } else if rest.starts_with("es") || rest.starts_with('s') || rest.starts_with('b') {
        -1
    } else {
        0
    };
    // "Bes" = B mol; sebagian orang menulis "Bes" untuk Bb, "B" untuk B natural.
    (((base + acc) % 12 + 12) % 12) as u8
}
