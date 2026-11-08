//! Tata letak partitur not angka gaya buku nyanyian jemaat (KJ, PKJ, NKB).
//!
//! Aturan gambar:
//! - garis harga (½, ¼ ketuk) di ATAS angka, satu garis per tingkat;
//! - titik oktaf tinggi di atas garis harga, titik oktaf rendah di bawah angka;
//! - legato/ikatan berupa lengkung di BAWAH angka;
//! - kres: garis miring "/" menembus angka, mol: "\";
//! - titik perpanjangan di tengah bawah, tanda napas ’ sesudah not;
//! - kepala: nomor, judul, "do = f   4 ketuk"; pencipta di bawah lagu.
//!
//! Satuan: point (1/72 inci).

use crate::font::{font, text_width, Style};
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Buku,
    Tayang,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Options {
    pub mode: Mode,
    pub page: String,
    pub note_size: f32,
    pub lyric_size: f32,
    /// Tampilkan petunjuk sunting (kotak kosong, tempat lirik, dll).
    pub editing: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Buku,
            page: "A4".into(),
            note_size: 15.0,
            lyric_size: 11.5,
            editing: false,
        }
    }
}

pub fn page_size(name: &str, mode: Mode) -> (f32, f32) {
    if mode == Mode::Tayang {
        return (960.0, 540.0);
    }
    let mm = 72.0 / 25.4;
    match name {
        "F4" => (215.0 * mm, 330.0 * mm),
        "Letter" => (612.0, 792.0),
        "A5" => (148.0 * mm, 210.0 * mm),
        _ => (210.0 * mm, 297.0 * mm),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Prim {
    Text {
        x: f32,
        y: f32,
        s: String,
        size: f32,
        style: Style,
        anchor: Anchor,
        faint: bool,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        w: f32,
    },
    Circle {
        x: f32,
        y: f32,
        r: f32,
    },
    /// Jalur tertutup diisi (Q = kurva kuadrat).
    Path {
        d: Vec<Seg>,
        stroke: Option<f32>,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    Z,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "k", rename_all = "camelCase")]
pub enum HitKind {
    Note { voice: usize, index: usize },
    Bar { voice: usize, index: usize },
    End { voice: usize, index: usize },
    Syllable { verse: u32, slot: usize },
    Meta { key: String },
    Text { number: u32 },
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    #[serde(flatten)]
    pub kind: HitKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub width: f32,
    pub height: f32,
    pub prims: Vec<Prim>,
    pub hits: Vec<Hit>,
}

impl Page {
    fn new(w: f32, h: f32) -> Self {
        Page {
            width: w,
            height: h,
            prims: Vec::new(),
            hits: Vec::new(),
        }
    }
}

pub fn layout(doc: &Doc, opts: &Options) -> Vec<Page> {
    let mut doc = doc.clone();
    crate::format::normalize(&mut doc);
    match opts.mode {
        Mode::Buku => layout_book(&doc, opts),
        Mode::Tayang => layout_slides(&doc, opts),
    }
}

// ---------------------------------------------------------------------------
// Ukuran-ukuran turunan dari besar huruf not.

#[derive(Clone, Copy)]
struct Metrics {
    fs: f32,
    ls: f32,
    digit_w: f32,
    cap: f32,
    beam_w: f32,
    beam_gap: f32,
    beam_step: f32,
    dot_r: f32,
    dot_gap: f32,
    dot_step: f32,
    thin: f32,
}

impl Metrics {
    fn new(fs: f32, ls: f32) -> Self {
        Metrics {
            fs,
            ls,
            digit_w: text_width("0", fs, Style::Regular),
            cap: font(Style::Regular).cap(fs),
            beam_w: (fs * 0.06).max(0.7),
            beam_gap: fs * 0.16,
            beam_step: fs * 0.15,
            dot_r: fs * 0.065,
            dot_gap: fs * 0.13,
            dot_step: fs * 0.17,
            thin: (fs * 0.045).max(0.6),
        }
    }

    /// Jumlah garis harga di atas angka: 1/2 ketuk = 1, 1/4 = 2, dst. Triol ikut tingkatnya.
    fn beam_levels(dur: u32) -> u32 {
        if dur >= BEAT {
            return 0;
        }
        for (k, div) in [(1, 3), (2, 6), (3, 12)] {
            if dur * div == BEAT {
                return k;
            }
        }
        let mut k = 0;
        let mut lim = BEAT;
        while dur < lim && k < 4 {
            lim /= 2;
            k += 1;
        }
        k
    }

    /// Tinggi di atas garis dasar yang dipakai satu not.
    fn above(&self, n: &Note) -> f32 {
        let mut h = self.cap;
        let lv = Self::beam_levels(n.dur);
        if lv > 0 {
            h += self.beam_gap + (lv - 1) as f32 * self.beam_step + self.beam_w;
        }
        if let Pitch::Tone { octave, .. } = n.pitch {
            if octave > 0 {
                h += self.dot_gap + octave as f32 * self.dot_step;
            }
        }
        if n.fermata {
            h += self.fs * 0.55;
        }
        if n.label.is_some() {
            h += self.fs * 0.85;
        }
        h
    }

    fn low_dots(&self, n: &Note) -> f32 {
        match n.pitch {
            Pitch::Tone { octave, .. } if octave < 0 => {
                self.dot_gap + (-octave) as f32 * self.dot_step
            }
            _ => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Kolom (slot) mendatar.

#[derive(Debug, Clone)]
struct Slot {
    t: u32,
    /// 0 garis birama, 1 not, 2 ujung suara (tempat menambah not).
    kind: u8,
    items: Vec<(usize, usize)>,
    bar: Option<BarKind>,
    width: f32,
    x: f32,
    cx: f32,
    line: usize,
}

struct Grid {
    slots: Vec<Slot>,
    /// (voice, item) -> indeks slot
    pos: std::collections::HashMap<(usize, usize), usize>,
    breaks: Vec<u32>,
}

fn build_grid(doc: &Doc, voices: &[usize], verses: &[usize], m: &Metrics, editing: bool) -> Grid {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<(u32, u8), Slot> = BTreeMap::new();
    let mut breaks = Vec::new();
    for &vi in voices {
        let v = &doc.voices[vi];
        let mut t = 0u32;
        for (ii, item) in v.items.iter().enumerate() {
            match item {
                Item::Note(n) => {
                    map.entry((t, 1))
                        .or_insert_with(|| new_slot(t, 1))
                        .items
                        .push((vi, ii));
                    t += n.dur;
                }
                Item::Bar(k, _) => {
                    let s = map.entry((t, 0)).or_insert_with(|| new_slot(t, 0));
                    s.items.push((vi, ii));
                    s.bar = Some(stronger(s.bar, *k));
                }
                Item::Break(_) => {
                    if vi == voices[0] {
                        breaks.push(t);
                    }
                }
            }
        }
        if editing {
            map.entry((t, 2))
                .or_insert_with(|| new_slot(t, 2))
                .items
                .push((vi, v.items.len()));
        }
    }
    let mut slots: Vec<Slot> = map.into_values().collect();
    let mut pos = std::collections::HashMap::new();
    for (si, s) in slots.iter().enumerate() {
        for &it in &s.items {
            pos.insert(it, si);
        }
    }

    // Lebar minimum tiap kolom.
    let n = slots.len();
    for i in 0..n {
        let next_t = slots[i + 1..]
            .iter()
            .find(|s| s.t > slots[i].t)
            .map(|s| s.t);
        let s = &mut slots[i];
        s.width = match s.kind {
            0 => {
                m.fs * match s.bar.unwrap_or(BarKind::Single) {
                    BarKind::Single => 0.8,
                    BarKind::Double => 0.95,
                    BarKind::Final => 1.05,
                    BarKind::RepeatStart | BarKind::RepeatEnd => 1.35,
                    BarKind::RepeatBoth => 1.7,
                }
            }
            2 => m.digit_w + m.fs * 0.9,
            _ => {
                let dt = next_t.map(|nt| nt - s.t).unwrap_or(BEAT);
                let mut pad = if dt >= BEAT {
                    m.fs * 0.68
                } else if dt * 2 >= BEAT {
                    m.fs * 0.42
                } else {
                    m.fs * 0.26
                };
                if let Some(nt) = next_t {
                    if dt < BEAT && nt % BEAT == 0 {
                        pad = pad.max(m.fs * 0.55);
                    }
                }
                let breath = s.items.iter().any(
                    |&(vi, ii)| matches!(&doc.voices[vi].items[ii], Item::Note(n) if n.breath),
                );
                m.digit_w + pad + if breath { m.fs * 0.3 } else { 0.0 }
            }
        };
    }

    // Lirik butuh ruang sampai suku kata berikutnya.
    if let Some(&mel) = voices.first() {
        let slots_idx = doc.lyric_slots();
        let hyph = text_width("-", m.ls, Style::Regular);
        for &vk in verses {
            let verse = &doc.verses[vk];
            let mut prev: Option<(usize, f32)> = None;
            for (k, &ii) in slots_idx.iter().enumerate() {
                let Some(Some(syl)) = verse.syllables.get(k) else {
                    continue;
                };
                let Some(&si) = pos.get(&(mel, ii)) else {
                    continue;
                };
                if let Some((psi, need)) = prev {
                    stretch(&mut slots, psi, si, need);
                }
                let w = text_width(&display_syllable(&syl.text), m.ls, Style::Regular);
                prev = Some((
                    si,
                    w + if syl.hyphen {
                        hyph + m.ls * 0.3
                    } else {
                        m.ls * 0.38
                    },
                ));
            }
        }
    }
    Grid { slots, pos, breaks }
}

fn new_slot(t: u32, kind: u8) -> Slot {
    Slot {
        t,
        kind,
        items: Vec::new(),
        bar: None,
        width: 0.0,
        x: 0.0,
        cx: 0.0,
        line: 0,
    }
}

fn stronger(a: Option<BarKind>, b: BarKind) -> BarKind {
    let rank = |k: BarKind| match k {
        BarKind::Single => 0,
        BarKind::Double => 1,
        BarKind::RepeatStart => 2,
        BarKind::RepeatEnd => 2,
        BarKind::Final => 3,
        BarKind::RepeatBoth => 4,
    };
    match a {
        Some(x) if rank(x) >= rank(b) => {
            if (x == BarKind::RepeatEnd && b == BarKind::RepeatStart)
                || (x == BarKind::RepeatStart && b == BarKind::RepeatEnd)
            {
                BarKind::RepeatBoth
            } else {
                x
            }
        }
        _ => b,
    }
}

fn stretch(slots: &mut [Slot], a: usize, b: usize, need: f32) {
    if b <= a {
        return;
    }
    let have: f32 = slots[a..b].iter().map(|s| s.width).sum();
    let deficit = need - have;
    if deficit <= 0.0 {
        return;
    }
    let idx: Vec<usize> = (a..b).filter(|&i| slots[i].kind == 1).collect();
    let list = if idx.is_empty() { vec![a] } else { idx };
    let share = deficit / list.len() as f32;
    for i in list {
        slots[i].width += share;
    }
}

fn display_syllable(s: &str) -> String {
    s.replace('~', " ")
}

// ---------------------------------------------------------------------------
// Pecah ke baris.

struct Line {
    from: usize,
    to: usize,
    right: f32,
}

fn break_lines(grid: &mut Grid, x0: f32, avail: f32) -> Vec<Line> {
    let slots = &mut grid.slots;
    let at_break = |t: u32| grid.breaks.contains(&t);
    // Unit = birama (diakhiri garis birama, kecuali |: yang membuka birama).
    let mut units: Vec<(usize, usize, bool)> = Vec::new();
    let mut start = 0;
    for i in 0..slots.len() {
        let s = &slots[i];
        if s.kind == 0 && s.bar == Some(BarKind::RepeatStart) {
            if i > start {
                units.push((start, i, at_break(s.t)));
            }
            start = i;
            continue;
        }
        if s.kind == 1
            && i > start
            && at_break(s.t)
            && !slots[start..i].iter().any(|x| x.kind == 0 && x.t == s.t)
        {
            units.push((start, i, true));
            start = i;
        }
        if s.kind == 0 {
            units.push((start, i + 1, at_break(s.t)));
            start = i + 1;
        }
    }
    if start < slots.len() {
        units.push((start, slots.len(), false));
    }

    let width =
        |a: usize, b: usize, slots: &[Slot]| slots[a..b].iter().map(|s| s.width).sum::<f32>();
    let mut lines: Vec<(usize, usize, bool)> = Vec::new();
    let mut cur: Option<(usize, usize)> = None;
    for (a, b, forced) in units {
        let w = width(a, b, slots);
        match cur {
            Some((ca, cb)) if width(ca, cb, slots) + w > avail => {
                lines.push((ca, cb, false));
                cur = Some((a, b));
            }
            Some((ca, _)) => cur = Some((ca, b)),
            None => cur = Some((a, b)),
        }
        if forced {
            let (ca, cb) = cur.take().unwrap();
            lines.push((ca, cb, true));
        }
    }
    if let Some((ca, cb)) = cur {
        lines.push((ca, cb, false));
    }

    let n = lines.len();
    let mut out = Vec::new();
    for (li, (a, b, _)) in lines.into_iter().enumerate() {
        let w = width(a, b, slots);
        let last = li + 1 == n;
        let mut extra = avail - w;
        if extra < 0.0 || w < avail * if last { 0.8 } else { 0.45 } {
            extra = 0.0;
        }
        let total_ev: f32 = slots[a..b]
            .iter()
            .filter(|s| s.kind == 1)
            .map(|s| s.width)
            .sum::<f32>()
            .max(1.0);
        let mut x = x0;
        for s in &mut slots[a..b] {
            let w = s.width
                + if s.kind == 1 {
                    extra * s.width / total_ev
                } else {
                    0.0
                };
            s.x = x;
            s.cx = if s.kind == 0 { x + w / 2.0 } else { x };
            s.line = li;
            s.width = w;
            x += w;
        }
        out.push(Line {
            from: a,
            to: b,
            right: x,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Satu sistem (baris partitur) dengan semua barisnya.

#[derive(Clone, Copy)]
enum RowKind {
    Voice(usize),
    Lyric(usize),
}

struct System {
    line: usize,
    rows: Vec<(RowKind, f32)>,
    height: f32,
}

fn measure_system(
    doc: &Doc,
    grid: &Grid,
    line: &Line,
    li: usize,
    voices: &[usize],
    verses: &[usize],
    m: &Metrics,
    editing: bool,
) -> System {
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for (k, &vi) in voices.iter().enumerate() {
        let name = doc.voices[vi].name.as_str();
        if k > 0 && (name == "T" || name == "B") {
            lower.push(vi);
        } else {
            upper.push(vi);
        }
    }
    let row_metrics = |vi: usize| -> (f32, f32) {
        let mut above = m.cap;
        let mut below = 0.0f32;
        let mut has_slur = false;
        for s in &grid.slots[line.from..line.to] {
            for &(v, ii) in &s.items {
                if v != vi {
                    continue;
                }
                if let Some(Item::Note(n)) = doc.voices[vi].items.get(ii) {
                    above = above.max(m.above(n));
                    below = below.max(m.low_dots(n));
                    if n.slur_start || n.slur_end {
                        has_slur = true;
                    }
                }
            }
        }
        if has_slur {
            below += m.fs * 0.42;
        }
        let _ = editing;
        (above + m.fs * 0.32, below + m.fs * 0.28)
    };
    let mut rows = Vec::new();
    let mut y = 0.0;
    for &vi in &upper {
        let (a, b) = row_metrics(vi);
        rows.push((RowKind::Voice(vi), y + a));
        y += a + b;
    }
    if !verses.is_empty() {
        y += m.ls * 0.15;
    }
    for &vk in verses {
        rows.push((RowKind::Lyric(vk), y + m.ls * 0.95));
        y += m.ls * 1.32;
    }
    if !verses.is_empty() {
        y += m.ls * 0.2;
    }
    for &vi in &lower {
        let (a, b) = row_metrics(vi);
        rows.push((RowKind::Voice(vi), y + a));
        y += a + b;
    }
    System {
        line: li,
        rows,
        height: y,
    }
}

// ---------------------------------------------------------------------------
// Penggambar.

struct Painter<'a> {
    page: &'a mut Page,
    m: Metrics,
}

impl<'a> Painter<'a> {
    fn text(&mut self, x: f32, y: f32, s: &str, size: f32, style: Style, anchor: Anchor) {
        self.page.prims.push(Prim::Text {
            x,
            y,
            s: s.to_string(),
            size,
            style,
            anchor,
            faint: false,
        });
    }
    fn faint(&mut self, x: f32, y: f32, s: &str, size: f32, style: Style, anchor: Anchor) {
        self.page.prims.push(Prim::Text {
            x,
            y,
            s: s.to_string(),
            size,
            style,
            anchor,
            faint: true,
        });
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.page.prims.push(Prim::Rect { x, y, w, h });
    }
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32) {
        self.page.prims.push(Prim::Line { x1, y1, x2, y2, w });
    }
    fn dot(&mut self, x: f32, y: f32, r: f32) {
        self.page.prims.push(Prim::Circle { x, y, r });
    }
    fn hit(&mut self, kind: HitKind, x: f32, y: f32, w: f32, h: f32) {
        self.page.hits.push(Hit { kind, x, y, w, h });
    }
}

fn draw_system(
    doc: &Doc,
    grid: &Grid,
    lines: &[Line],
    sys: &System,
    y0: f32,
    x0: f32,
    pg: &mut Painter,
    open_slurs: &mut Vec<Option<(f32, f32)>>,
    editing: bool,
) {
    let m = pg.m;
    let line = &lines[sys.line];
    let slots = &grid.slots[line.from..line.to];
    let base_of = |vi: usize| {
        sys.rows
            .iter()
            .find_map(|(k, y)| matches!(k, RowKind::Voice(v) if *v == vi).then_some(y0 + *y))
    };
    let note_cx = |s: &Slot| s.x + m.digit_w / 2.0 + m.fs * 0.08;

    // Garis birama per kelompok suara (atas & bawah lirik).
    let mut groups: Vec<Vec<f32>> = vec![Vec::new()];
    for (k, _) in &sys.rows {
        match k {
            RowKind::Voice(vi) => groups.last_mut().unwrap().push(base_of(*vi).unwrap()),
            RowKind::Lyric(_) => {
                if !groups.last().unwrap().is_empty() {
                    groups.push(Vec::new());
                }
            }
        }
    }
    groups.retain(|g| !g.is_empty());
    for s in slots.iter().filter(|s| s.kind == 0) {
        for g in &groups {
            let top = g[0] - m.cap - m.fs * 0.12;
            let bot = g[g.len() - 1] + m.fs * 0.12;
            draw_bar(pg, s.cx, top, bot, s.bar.unwrap_or(BarKind::Single), g);
        }
        for &(vi, ii) in &s.items {
            if let Some(b) = base_of(vi) {
                pg.hit(
                    HitKind::Bar {
                        voice: vi,
                        index: ii,
                    },
                    s.cx - m.fs * 0.3,
                    b - m.cap - m.fs * 0.3,
                    m.fs * 0.6,
                    m.cap + m.fs * 0.6,
                );
            }
        }
    }

    for (rk, _) in &sys.rows {
        let RowKind::Voice(vi) = *rk else { continue };
        let base = base_of(vi).unwrap();
        let voice = &doc.voices[vi];
        let evs: Vec<(usize, &Note, &Slot)> = slots
            .iter()
            .filter(|s| s.kind == 1)
            .flat_map(|s| {
                s.items
                    .iter()
                    .filter(|it| it.0 == vi)
                    .map(move |&(_, ii)| (ii, s))
            })
            .filter_map(|(ii, s)| match &voice.items[ii] {
                Item::Note(n) => Some((ii, n, s)),
                _ => None,
            })
            .collect();

        // Garis harga di atas angka: per ketuk.
        let mut i = 0;
        while i < evs.len() {
            let beat = evs[i].2.t / BEAT;
            let mut j = i;
            while j < evs.len() && evs[j].2.t / BEAT == beat && evs[j].1.dur < BEAT {
                j += 1;
            }
            if j > i {
                let grp = &evs[i..j];
                for level in 1..=4u32 {
                    let mut run: Vec<f32> = Vec::new();
                    let flush = |run: &mut Vec<f32>, pg: &mut Painter| {
                        if let (Some(a), Some(b)) = (run.first(), run.last()) {
                            let xa = a - m.digit_w * 0.55;
                            let xb = b + m.digit_w * 0.55;
                            let y = base
                                - m.cap
                                - m.beam_gap
                                - (level - 1) as f32 * m.beam_step
                                - m.beam_w;
                            pg.rect(xa, y, xb - xa, m.beam_w);
                        }
                        run.clear();
                    };
                    for (_, n, s) in grp {
                        if Metrics::beam_levels(n.dur) >= level {
                            run.push(note_cx(s));
                        } else {
                            flush(&mut run, pg);
                        }
                    }
                    flush(&mut run, pg);
                }
                // Angka 3 untuk triol.
                if grp.iter().all(|e| e.1.dur * 3 == BEAT)
                    || grp.iter().all(|e| e.1.dur * 6 == BEAT)
                {
                    let xa = note_cx(grp[0].2);
                    let xb = note_cx(grp[grp.len() - 1].2);
                    let lv = Metrics::beam_levels(grp[0].1.dur) as f32;
                    pg.text(
                        (xa + xb) / 2.0,
                        base - m.cap - m.beam_gap - lv * m.beam_step - m.fs * 0.12,
                        "3",
                        m.fs * 0.5,
                        Style::Italic,
                        Anchor::Middle,
                    );
                }
                i = j;
            } else {
                i += 1;
            }
        }

        for &(ii, n, s) in &evs {
            let cx = note_cx(s);
            draw_note(pg, n, cx, base);
            let top = base - m.above(n) - m.fs * 0.1;
            let hw = (s.width * 0.5).max(m.digit_w * 0.75);
            pg.hit(
                HitKind::Note {
                    voice: vi,
                    index: ii,
                },
                cx - hw * 0.8,
                top,
                hw * 1.6,
                base - top + m.fs * 0.35 + m.low_dots(n),
            );
        }

        // Ujung suara (tempat menambah not).
        if editing {
            for s in slots.iter().filter(|s| s.kind == 2) {
                for &(v, ii) in &s.items {
                    if v == vi {
                        let cx = note_cx(s);
                        pg.faint(cx, base, "+", m.fs * 0.9, Style::Regular, Anchor::Middle);
                        pg.hit(
                            HitKind::End {
                                voice: vi,
                                index: ii,
                            },
                            cx - m.digit_w,
                            base - m.cap - m.fs * 0.3,
                            m.digit_w * 2.0,
                            m.cap + m.fs * 0.6,
                        );
                    }
                }
            }
        }

        // Legato di bawah angka.
        let slur_y = |n: &Note| base + m.fs * 0.2 + m.low_dots(n);
        let mut open = open_slurs[vi].take();
        for &(_, n, s) in &evs {
            if !n.is_tone() {
                continue;
            }
            if n.slur_start {
                open = Some((note_cx(s), slur_y(n)));
            }
            if n.slur_end {
                let (xa, ya, from_prev) = match open {
                    Some((x, y)) if x <= note_cx(s) && !(x < slots[0].x) => (x, y, false),
                    Some((_, y)) => (slots[0].x - m.fs * 0.1, y, true),
                    None => (slots[0].x - m.fs * 0.1, slur_y(n), true),
                };
                let _ = from_prev;
                draw_slur(pg, xa, note_cx(s), ya.max(slur_y(n)));
                open = None;
            }
        }
        if let Some((x, y)) = open {
            let xa = if x >= slots[0].x { x } else { slots[0].x };
            draw_slur(pg, xa, line.right + m.fs * 0.1, y);
            // Lanjutkan di baris berikutnya dari tepi kiri.
            open_slurs[vi] = Some((f32::MIN, y));
        }
    }

    // Lirik.
    let slots_idx = doc.lyric_slots();
    let mel = sys.rows.iter().find_map(|(k, _)| {
        if let RowKind::Voice(v) = k {
            Some(*v)
        } else {
            None
        }
    });
    let hyph_w = text_width("-", m.ls, Style::Regular);
    for (rk, ry) in &sys.rows {
        let RowKind::Lyric(vk) = *rk else { continue };
        let Some(mel) = mel else { continue };
        let verse = &doc.verses[vk];
        let by = y0 + ry;
        pg.text(
            x0 - m.ls * 0.45,
            by,
            &format!("{}.", verse.number),
            m.ls,
            Style::Regular,
            Anchor::End,
        );
        // Suku kata di baris ini.
        let mut items: Vec<(usize, f32, Option<&Syllable>)> = Vec::new();
        for (k, &ii) in slots_idx.iter().enumerate() {
            let Some(&si) = grid.pos.get(&(mel, ii)) else {
                continue;
            };
            if si < line.from || si >= line.to {
                continue;
            }
            let s = &grid.slots[si];
            let lx = note_cx(s) - m.digit_w / 2.0;
            items.push((k, lx, verse.syllables.get(k).and_then(|x| x.as_ref())));
        }
        for (j, &(k, lx, syl)) in items.iter().enumerate() {
            match syl {
                Some(syl) => {
                    let t = display_syllable(&syl.text);
                    let w = text_width(&t, m.ls, Style::Regular);
                    pg.text(lx, by, &t, m.ls, Style::Regular, Anchor::Start);
                    if syl.hyphen {
                        let end = lx + w;
                        let mut hx = end + m.ls * 0.06;
                        if let Some(&(_, nx, _)) = items.get(j + 1) {
                            if nx - end > hyph_w * 1.3 {
                                hx = (end + nx) / 2.0 - hyph_w / 2.0;
                            }
                        }
                        pg.text(hx, by, "-", m.ls, Style::Regular, Anchor::Start);
                    }
                    pg.hit(
                        HitKind::Syllable {
                            verse: verse.number,
                            slot: k,
                        },
                        lx - 1.0,
                        by - m.ls * 0.9,
                        w.max(m.digit_w * 1.2) + 2.0,
                        m.ls * 1.2,
                    );
                }
                None => {
                    if editing {
                        pg.faint(lx, by, "_", m.ls, Style::Regular, Anchor::Start);
                        pg.hit(
                            HitKind::Syllable {
                                verse: verse.number,
                                slot: k,
                            },
                            lx - 1.0,
                            by - m.ls * 0.9,
                            m.digit_w * 1.4,
                            m.ls * 1.2,
                        );
                    }
                }
            }
        }
    }
}

fn draw_note(pg: &mut Painter, n: &Note, cx: f32, base: f32) {
    let m = pg.m;
    match n.pitch {
        Pitch::Hold => pg.dot(cx, base - m.cap * 0.32, m.fs * 0.075),
        Pitch::Rest => pg.text(cx, base, "0", m.fs, Style::Regular, Anchor::Middle),
        Pitch::Tone {
            degree,
            octave,
            accidental,
        } => {
            pg.text(
                cx,
                base,
                &degree.to_string(),
                m.fs,
                Style::Regular,
                Anchor::Middle,
            );
            let lv = Metrics::beam_levels(n.dur);
            let above_beams = if lv > 0 {
                m.beam_gap + (lv - 1) as f32 * m.beam_step + m.beam_w
            } else {
                0.0
            };
            for k in 0..octave.max(0) {
                pg.dot(
                    cx,
                    base - m.cap - above_beams - m.dot_gap - m.dot_r - k as f32 * m.dot_step,
                    m.dot_r,
                );
            }
            for k in 0..(-octave).max(0) {
                pg.dot(
                    cx,
                    base + m.dot_gap + m.dot_r - m.fs * 0.03 + k as f32 * m.dot_step,
                    m.dot_r,
                );
            }
            if accidental != 0 {
                let xl = cx - m.digit_w * 0.55;
                let xr = cx + m.digit_w * 0.55;
                let yb = base + m.fs * 0.04;
                let yt = base - m.cap - m.fs * 0.04;
                let (y1, y2) = if accidental > 0 { (yb, yt) } else { (yt, yb) };
                pg.line(xl, y1, xr, y2, m.fs * 0.065);
            }
        }
    }
    if n.breath {
        pg.text(
            cx + m.digit_w * 0.6,
            base + m.cap * 0.12,
            "\u{2019}",
            m.fs * 1.05,
            Style::Regular,
            Anchor::Start,
        );
    }
    let mut top = base - m.cap;
    let lv = Metrics::beam_levels(n.dur);
    if lv > 0 {
        top -= m.beam_gap + (lv - 1) as f32 * m.beam_step + m.beam_w;
    }
    if let Pitch::Tone { octave, .. } = n.pitch {
        if octave > 0 {
            top -= m.dot_gap + octave as f32 * m.dot_step;
        }
    }
    if n.fermata {
        let r = m.fs * 0.3;
        let fy = top - m.fs * 0.12;
        pg.page.prims.push(Prim::Path {
            d: vec![Seg::M(cx - r, fy), Seg::Q(cx, fy - r * 1.9, cx + r, fy)],
            stroke: Some(m.fs * 0.06),
        });
        pg.dot(cx, fy - m.fs * 0.08, m.fs * 0.06);
        top = fy - r;
    }
    if let Some(l) = &n.label {
        pg.text(
            cx - m.digit_w / 2.0,
            top - m.fs * 0.25,
            l,
            m.fs * 0.62,
            Style::Italic,
            Anchor::Start,
        );
    }
}

fn draw_slur(pg: &mut Painter, xa: f32, xb: f32, y: f32) {
    let m = pg.m;
    let len = (xb - xa).max(m.fs * 0.5);
    let h = (m.fs * 0.16 + len * 0.07).min(m.fs * 0.42);
    let t = m.fs * 0.055;
    let mx = (xa + xb) / 2.0;
    pg.page.prims.push(Prim::Path {
        d: vec![
            Seg::M(xa, y),
            Seg::Q(mx, y + h * 2.0, xb, y),
            Seg::Q(mx, y + h * 2.0 - t * 2.0, xa, y),
            Seg::Z,
        ],
        stroke: None,
    });
}

fn draw_bar(pg: &mut Painter, x: f32, top: f32, bot: f32, kind: BarKind, bases: &[f32]) {
    let m = pg.m;
    let thick = m.fs * 0.16;
    let u = m.fs * 0.1;
    let line = |pg: &mut Painter, xx: f32, w: f32| pg.rect(xx - w / 2.0, top, w, bot - top);
    let dots = |pg: &mut Painter, xx: f32| {
        for b in bases {
            pg.dot(xx, b - m.cap * 0.5 - m.fs * 0.14, m.fs * 0.065);
            pg.dot(xx, b - m.cap * 0.5 + m.fs * 0.14, m.fs * 0.065);
        }
    };
    match kind {
        BarKind::Single => line(pg, x, m.thin),
        BarKind::Double => {
            line(pg, x - u, m.thin);
            line(pg, x + u, m.thin);
        }
        BarKind::Final => {
            line(pg, x - u * 1.3, m.thin);
            line(pg, x + u * 0.6, thick);
        }
        BarKind::RepeatStart => {
            line(pg, x - u * 2.0, thick);
            line(pg, x - u * 0.2, m.thin);
            dots(pg, x + u * 1.7);
        }
        BarKind::RepeatEnd => {
            dots(pg, x - u * 1.7);
            line(pg, x + u * 0.2, m.thin);
            line(pg, x + u * 2.0, thick);
        }
        BarKind::RepeatBoth => {
            dots(pg, x - u * 3.2);
            line(pg, x - u * 1.6, m.thin);
            line(pg, x, thick);
            line(pg, x + u * 1.6, m.thin);
            dots(pg, x + u * 3.2);
        }
    }
}

// ---------------------------------------------------------------------------
// Halaman buku.

fn key_text(meta: &Meta) -> String {
    let mut parts = Vec::new();
    if let Some(k) = &meta.key {
        let k = k.trim();
        let k = k.rsplit('=').next().unwrap_or(k).trim();
        parts.push(format!("do = {}", k.to_lowercase()));
    }
    if let Some(b) = &meta.beats {
        if b.contains('/') {
            parts.push(b.trim().to_string());
        } else {
            parts.push(format!("{} ketuk", b.trim()));
        }
    }
    parts.join("    ")
}

fn wrap_text(text: &str, size: f32, style: Style, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split_whitespace() {
            let cand = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", cur, word)
            };
            if text_width(&cand, size, style) > width && !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            } else {
                cur = cand;
            }
        }
        lines.push(cur);
    }
    lines
}

fn credits(meta: &Meta) -> Vec<String> {
    let mut v = Vec::new();
    let mut push = |label: &str, val: &Option<String>| {
        if let Some(s) = val {
            v.push(format!("{}: {}", label, s));
        }
    };
    push("Syair", &meta.lyricist);
    push("Lagu", &meta.composer);
    push("Terjemahan", &meta.translator);
    push("Aransemen", &meta.arranger);
    v
}

fn layout_book(doc: &Doc, opts: &Options) -> Vec<Page> {
    let (pw, ph) = page_size(&opts.page, Mode::Buku);
    let m = Metrics::new(opts.note_size, opts.lyric_size);
    let margin = 15.0 * 72.0 / 25.4;
    let voices: Vec<usize> = (0..doc.voices.len()).collect();
    let verses: Vec<usize> = (0..doc.verses.len()).collect();
    let prefix = if verses.is_empty() {
        0.0
    } else {
        text_width("10.", m.ls, Style::Regular) + m.ls * 0.6
    };
    let x0 = margin + prefix;
    let avail = pw - margin - x0;
    let editing = opts.editing;

    let mut pages = vec![Page::new(pw, ph)];
    let mut y = margin;
    let meta = &doc.meta;

    // Kepala lagu.
    {
        let mut pg = Painter {
            page: &mut pages[0],
            m,
        };
        let title_size = m.fs * 1.3;
        let num_size = m.fs * 1.25;
        let small = m.ls * 0.92;
        let top = y + title_size;
        match &meta.number {
            Some(n) => pg.text(margin, top, n, num_size, Style::Bold, Anchor::Start),
            None if editing => pg.faint(margin, top, "Nomor", num_size, Style::Bold, Anchor::Start),
            None => {}
        }
        let nw = meta
            .number
            .as_deref()
            .map(|n| text_width(n, num_size, Style::Bold))
            .unwrap_or(text_width("Nomor", num_size, Style::Bold));
        pg.hit(
            HitKind::Meta {
                key: "nomor".into(),
            },
            margin - 2.0,
            top - num_size,
            nw + 4.0,
            num_size * 1.25,
        );
        match &meta.title {
            Some(t) => pg.text(pw / 2.0, top, t, title_size, Style::Bold, Anchor::Middle),
            None if editing => pg.faint(
                pw / 2.0,
                top,
                "Judul lagu",
                title_size,
                Style::Bold,
                Anchor::Middle,
            ),
            None => {}
        }
        let tw = text_width(
            meta.title.as_deref().unwrap_or("Judul lagu"),
            title_size,
            Style::Bold,
        )
        .max(120.0);
        pg.hit(
            HitKind::Meta {
                key: "judul".into(),
            },
            pw / 2.0 - tw / 2.0 - 4.0,
            top - title_size,
            tw + 8.0,
            title_size * 1.3,
        );
        y = top + title_size * 0.35;
        if meta.subtitle.is_some() || editing {
            let s = meta.subtitle.clone();
            y += small * 1.1;
            match &s {
                Some(s) => pg.text(pw / 2.0, y, s, small, Style::Italic, Anchor::Middle),
                None => pg.faint(
                    pw / 2.0,
                    y,
                    "Ayat atau keterangan",
                    small,
                    Style::Italic,
                    Anchor::Middle,
                ),
            }
            let sw = text_width(
                s.as_deref().unwrap_or("Ayat atau keterangan"),
                small,
                Style::Italic,
            );
            pg.hit(
                HitKind::Meta { key: "ayat".into() },
                pw / 2.0 - sw / 2.0 - 4.0,
                y - small,
                sw + 8.0,
                small * 1.3,
            );
        }
        y += m.ls * 1.5;
        // Kiri: tempo. Kanan: do = ..., ketuk.
        let kt = key_text(meta);
        if !kt.is_empty() {
            pg.text(pw - margin, y, &kt, m.ls, Style::Bold, Anchor::End);
        } else if editing {
            pg.faint(
                pw - margin,
                y,
                "do = c    4 ketuk",
                m.ls,
                Style::Bold,
                Anchor::End,
            );
        }
        let kw = text_width(
            if kt.is_empty() {
                "do = c    4 ketuk"
            } else {
                &kt
            },
            m.ls,
            Style::Bold,
        );
        pg.hit(
            HitKind::Meta { key: "nada".into() },
            pw - margin - kw - 4.0,
            y - m.ls,
            kw + 8.0,
            m.ls * 1.3,
        );
        match &meta.tempo {
            Some(t) => pg.text(margin, y, t, m.ls, Style::Italic, Anchor::Start),
            None if editing => pg.faint(margin, y, "Tempo", m.ls, Style::Italic, Anchor::Start),
            None => {}
        }
        let tw = text_width(
            meta.tempo.as_deref().unwrap_or("Tempo"),
            m.ls,
            Style::Italic,
        );
        pg.hit(
            HitKind::Meta {
                key: "tempo".into(),
            },
            margin - 2.0,
            y - m.ls,
            tw + 4.0,
            m.ls * 1.3,
        );
        y += m.fs * 1.0;
    }

    // Partitur.
    let mut systems = Vec::new();
    let mut grid = None;
    let mut lines = Vec::new();
    if !doc.voices.is_empty() {
        let mut g = build_grid(doc, &voices, &verses, &m, editing);
        lines = break_lines(&mut g, x0, avail);
        for (li, ln) in lines.iter().enumerate() {
            systems.push(measure_system(
                doc, &g, ln, li, &voices, &verses, &m, editing,
            ));
        }
        grid = Some(g);
    } else if editing {
        let mut pg = Painter {
            page: &mut pages[0],
            m,
        };
        pg.faint(
            x0,
            y + m.fs * 1.5,
            "Klik di sini lalu ketik angka 1 sampai 7 untuk mulai menulis not.",
            m.ls,
            Style::Italic,
            Anchor::Start,
        );
        pg.hit(
            HitKind::End { voice: 0, index: 0 },
            x0,
            y,
            avail,
            m.fs * 2.5,
        );
        y += m.fs * 3.0;
    }
    let gap = if doc
        .voices
        .iter()
        .skip(1)
        .any(|v| v.name == "T" || v.name == "B")
    {
        m.fs * 1.4
    } else {
        m.fs * 0.75
    };
    let bottom = ph - margin;
    let mut open_slurs: Vec<Option<(f32, f32)>> = vec![None; doc.voices.len()];
    for sys in &systems {
        if y + sys.height > bottom && y > margin + 1.0 {
            pages.push(Page::new(pw, ph));
            y = margin;
        }
        let page = pages.last_mut().unwrap();
        let mut pg = Painter { page, m };
        draw_system(
            doc,
            grid.as_ref().unwrap(),
            &lines,
            sys,
            y,
            x0,
            &mut pg,
            &mut open_slurs,
            editing,
        );
        y += sys.height + gap;
    }

    // Sesudah partitur: pencipta, bait teks, catatan.
    let small = m.ls * 0.88;
    let mut blocks: Vec<(Vec<(String, Style, f32, Option<HitKind>, bool)>, f32)> = Vec::new();
    let cr = credits(meta);
    if !cr.is_empty() || editing {
        let mut b = Vec::new();
        if cr.is_empty() {
            b.push((
                "Syair: …   Lagu: …".to_string(),
                Style::Regular,
                small,
                Some(HitKind::Meta {
                    key: "syair".into(),
                }),
                true,
            ));
        }
        for c in &cr {
            let key = if c.starts_with("Syair") {
                "syair"
            } else if c.starts_with("Lagu") {
                "lagu"
            } else if c.starts_with("Terjemahan") {
                "terjemahan"
            } else {
                "aransemen"
            };
            b.push((
                c.clone(),
                Style::Regular,
                small,
                Some(HitKind::Meta { key: key.into() }),
                false,
            ));
        }
        blocks.push((b, small * 1.35));
    }
    for t in &doc.texts {
        let mut b = Vec::new();
        let indent = text_width("10. ", m.ls, Style::Regular);
        let body = t.lines.join("\n");
        let wrapped = wrap_text(&body, m.ls, Style::Regular, pw - 2.0 * margin - indent);
        for (i, l) in wrapped.iter().enumerate() {
            let s = if i == 0 {
                format!("{}.\t{}", t.number, l)
            } else {
                format!("\t{}", l)
            };
            b.push((
                s,
                Style::Regular,
                m.ls,
                Some(HitKind::Text { number: t.number }),
                false,
            ));
        }
        blocks.push((b, m.ls * 1.4));
    }
    if let Some(n) = &meta.note {
        let b = wrap_text(n, small, Style::Italic, pw - 2.0 * margin)
            .into_iter()
            .map(|l| {
                (
                    l,
                    Style::Italic,
                    small,
                    Some(HitKind::Meta {
                        key: "catatan".into(),
                    }),
                    false,
                )
            })
            .collect();
        blocks.push((b, small * 1.35));
    }
    if !blocks.is_empty() {
        y += m.fs * 0.2;
    }
    for (b, lh) in blocks {
        let h = b.len() as f32 * lh + m.fs * 0.5;
        if y + h > bottom {
            pages.push(Page::new(pw, ph));
            y = margin;
        }
        let page = pages.last_mut().unwrap();
        let mut pg = Painter { page, m };
        let indent = text_width("10. ", m.ls, Style::Regular);
        for (s, style, size, hit, faint) in b {
            y += lh;
            let (lead, rest) = match s.split_once('\t') {
                Some((a, r)) => (a.to_string(), r.to_string()),
                None => (String::new(), s.clone()),
            };
            if !lead.is_empty() {
                pg.text(margin, y, &lead, size, style, Anchor::Start);
            }
            let x = if s.contains('\t') {
                margin + indent
            } else {
                margin
            };
            if faint {
                pg.faint(x, y, &rest, size, style, Anchor::Start);
            } else {
                pg.text(x, y, &rest, size, style, Anchor::Start);
            }
            if let Some(k) = hit {
                let w = text_width(&rest, size, style).max(60.0);
                pg.hit(k, x - 2.0, y - size, w + 4.0 + (x - margin), size * 1.3);
            }
        }
        y += m.fs * 0.5;
    }

    let n = pages.len();
    if n > 1 {
        for (i, p) in pages.iter_mut().enumerate() {
            let mut pg = Painter { page: p, m };
            pg.text(
                pw / 2.0,
                ph - margin / 2.0,
                &format!("{} dari {}", i + 1, n),
                small,
                Style::Regular,
                Anchor::Middle,
            );
        }
    }
    pages
}

// ---------------------------------------------------------------------------
// Mode tayang: satu bait per rangkaian slide 16:9, untuk proyektor.

fn layout_slides(doc: &Doc, opts: &Options) -> Vec<Page> {
    let (pw, ph) = page_size("", Mode::Tayang);
    let m = Metrics::new(opts.note_size * 2.0, opts.lyric_size * 2.2);
    let margin = 40.0;
    let mut pages = Vec::new();
    let meta = &doc.meta;
    let head_h = 64.0;

    let header = |pg: &mut Painter, verse: Option<u32>| {
        let title = [meta.number.clone(), meta.title.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("   ");
        pg.text(
            margin,
            margin + 18.0,
            &title,
            24.0,
            Style::Bold,
            Anchor::Start,
        );
        let kt = key_text(meta);
        pg.text(
            pw - margin,
            margin + 18.0,
            &kt,
            18.0,
            Style::Bold,
            Anchor::End,
        );
        if let Some(v) = verse {
            pg.text(
                pw - margin,
                margin + 42.0,
                &format!("bait {}", v),
                16.0,
                Style::Italic,
                Anchor::End,
            );
        }
        pg.rect(
            margin,
            margin + 30.0,
            pw - 2.0 * margin - if verse.is_some() { 80.0 } else { 0.0 },
            1.2,
        );
    };

    let voices: Vec<usize> = if doc.voices.is_empty() {
        vec![]
    } else {
        vec![0]
    };
    let verse_sets: Vec<Vec<usize>> = if doc.verses.is_empty() {
        vec![vec![]]
    } else {
        (0..doc.verses.len()).map(|i| vec![i]).collect()
    };

    for vs in verse_sets {
        if voices.is_empty() {
            break;
        }
        let prefix = if vs.is_empty() {
            0.0
        } else {
            text_width("10.", m.ls, Style::Regular) + m.ls * 0.5
        };
        let x0 = margin + prefix;
        let avail = pw - margin - x0;
        let mut g = build_grid(doc, &voices, &vs, &m, false);
        let lines = break_lines(&mut g, x0, avail);
        let systems: Vec<System> = lines
            .iter()
            .enumerate()
            .map(|(li, ln)| measure_system(doc, &g, ln, li, &voices, &vs, &m, false))
            .collect();
        let verse_no = vs.first().map(|&k| doc.verses[k].number);
        let mut page = Page::new(pw, ph);
        {
            let mut pg = Painter { page: &mut page, m };
            header(&mut pg, verse_no);
        }
        let mut y = margin + head_h;
        let mut open = vec![None; doc.voices.len()];
        let gap = m.fs * 0.6;
        for sys in &systems {
            if y + sys.height > ph - margin && y > margin + head_h + 1.0 {
                pages.push(std::mem::replace(&mut page, Page::new(pw, ph)));
                let mut pg = Painter { page: &mut page, m };
                header(&mut pg, verse_no);
                y = margin + head_h;
            }
            let mut pg = Painter { page: &mut page, m };
            draw_system(doc, &g, &lines, sys, y, x0, &mut pg, &mut open, false);
            y += sys.height + gap;
        }
        pages.push(page);
    }

    // Bait teks sebagai slide teks.
    for t in &doc.texts {
        let mut page = Page::new(pw, ph);
        let mut pg = Painter { page: &mut page, m };
        header(&mut pg, Some(t.number));
        let size = 30.0;
        let mut y = margin + head_h + size;
        for l in wrap_text(&t.lines.join("\n"), size, Style::Regular, pw - 2.0 * margin) {
            pg.text(margin, y, &l, size, Style::Regular, Anchor::Start);
            y += size * 1.35;
        }
        pages.push(page);
    }
    if pages.is_empty() {
        let mut page = Page::new(pw, ph);
        let mut pg = Painter { page: &mut page, m };
        header(&mut pg, None);
        pages.push(page);
    }
    pages
}
