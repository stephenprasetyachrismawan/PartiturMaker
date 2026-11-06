use notangka::model::*;
use notangka::{format, parse, Severity};

const MULIAKAN: &str = include_str!("../examples-nada/muliakan.nada");
const FAJAR: &str = include_str!("../examples-nada/fajar.nada");

fn errors(src: &str) -> Vec<String> {
    parse(src)
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| format!("{}:{} {}", d.line, d.col, d.message))
        .collect()
}

fn notes(doc: &Doc, v: usize) -> Vec<(Pitch, u32)> {
    doc.voices[v].notes().map(|n| (n.pitch, n.dur)).collect()
}

#[test]
fn contoh_bebas_kesalahan_dan_peringatan() {
    for src in [MULIAKAN, FAJAR] {
        let p = parse(src);
        let all: Vec<String> = p
            .diagnostics
            .iter()
            .map(|d| format!("{}:{} {:?} {}", d.line, d.col, d.severity, d.message))
            .collect();
        assert!(all.is_empty(), "{:#?}", all);
    }
}

#[test]
fn kurung_membagi_ketuk() {
    let d = parse("suara S { 1 [2 3] [4 [5 6]] [7 6 5] }").doc;
    let durs: Vec<u32> = notes(&d, 0).iter().map(|x| x.1).collect();
    assert_eq!(durs, vec![960, 480, 480, 480, 240, 240, 320, 320, 320]);
}

#[test]
fn titik_dan_napas_dan_label() {
    let d = parse("suara S { @Refrein 1 . ; [. 2] }").doc;
    let v = &d.voices[0];
    let ns: Vec<&Note> = v.notes().collect();
    assert_eq!(ns[0].label.as_deref(), Some("Refrein"));
    assert_eq!(ns[1].pitch, Pitch::Hold);
    assert!(ns[1].breath);
    assert_eq!(ns[2].dur, 480);
}

#[test]
fn oktaf_kres_mol_fermata() {
    let d = parse("suara S { 1' 5,, 4# 7b 3^ }").doc;
    let ns: Vec<&Note> = d.voices[0].notes().collect();
    assert_eq!(
        ns[0].pitch,
        Pitch::Tone {
            degree: 1,
            octave: 1,
            accidental: 0
        }
    );
    assert_eq!(
        ns[1].pitch,
        Pitch::Tone {
            degree: 5,
            octave: -2,
            accidental: 0
        }
    );
    assert_eq!(
        ns[2].pitch,
        Pitch::Tone {
            degree: 4,
            octave: 0,
            accidental: 1
        }
    );
    assert_eq!(
        ns[3].pitch,
        Pitch::Tone {
            degree: 7,
            octave: 0,
            accidental: -1
        }
    );
    assert!(ns[4].fermata);
}

#[test]
fn legato_satu_suku_kata() {
    let d = parse("suara S { 1 (2 3) 4 }\nbait 1 { a b c }").doc;
    assert_eq!(d.lyric_slots().len(), 3);
}

#[test]
fn kesalahan_punya_baris_dan_saran() {
    let p = parse("judul = \"X\"\nsuara S { 1 2 - 3 }\nwarna = merah\nsuara Q { 1 }\n");
    let e: Vec<_> = p
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(e
        .iter()
        .any(|d| d.line == 2 && d.hint.as_deref().unwrap_or("").contains("titik")));
    assert!(e.iter().any(|d| d.line == 3 && d.message.contains("warna")));
    assert!(e.iter().any(|d| d.line == 4 && d.message.contains("Q")));
    assert!(!errors("suara S { [1 2 }").is_empty());
    assert!(!errors("suara S { 1 (2 3 }").is_empty());
}

#[test]
fn peringatan_birama_dan_suara() {
    let p = parse("ketuk = 4\nsuara S { 1 | 1 2 3 | 1 2 3 4 | 5 }\nsuara A { 1 2 }");
    let w: Vec<_> = p
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .map(|d| d.message.clone())
        .collect();
    assert!(
        w.iter().any(|m| m.contains("Birama ini 3 ketuk")),
        "{:?}",
        w
    );
    assert!(w.iter().any(|m| m.contains("Suara A")), "{:?}", w);
}

#[test]
fn format_idempoten_dan_setara() {
    for src in [
        MULIAKAN,
        FAJAR,
        "suara S { 1 [. 2] [3 4 5 6] [[1 2] 3] [5 6 7] }",
    ] {
        let a = parse(src).doc;
        let f1 = format(&a);
        let b = parse(&f1);
        assert!(!b.has_errors(), "{}\n{:?}", f1, b.diagnostics);
        let f2 = format(&b.doc);
        assert_eq!(f1, f2, "formatter tidak idempoten");
        for (va, vb) in a.voices.iter().zip(b.doc.voices.iter()) {
            assert_eq!(
                va.notes().map(|n| (n.pitch, n.dur)).collect::<Vec<_>>(),
                vb.notes().map(|n| (n.pitch, n.dur)).collect::<Vec<_>>()
            );
        }
        assert_eq!(a.verses, b.doc.verses);
    }
}

#[test]
fn format_merapikan_tulisan_berantakan() {
    let src = "judul=\"Lagu\"   \n nada= g\nsuara   sopran{1  2 [3   4]|5 . . .|}\nbait 1{Ha-le-lu-ya a-min}";
    let f = format(&parse(src).doc);
    assert_eq!(f, "judul = \"Lagu\"\nnada  = g\n\nsuara S {\n  1 2 [3 4] | 5 . . . |\n}\n\nbait 1 {\n  Ha-le-lu-ya a-min\n}\n");
}

#[test]
fn normalisasi_memecah_not_lintas_ketuk() {
    let mut d = parse("suara S { 1 }").doc;
    d.voices[0].items = vec![
        Item::Note(Note::new(
            Pitch::Tone {
                degree: 1,
                octave: 0,
                accidental: 0,
            },
            480,
        )),
        Item::Note(Note::new(
            Pitch::Tone {
                degree: 2,
                octave: 0,
                accidental: 0,
            },
            960,
        )),
        Item::Note(Note::new(
            Pitch::Tone {
                degree: 3,
                octave: 0,
                accidental: 0,
            },
            480,
        )),
    ];
    let f = format(&d);
    assert!(f.contains("[1 2] [. 3]"), "{}", f);
}
