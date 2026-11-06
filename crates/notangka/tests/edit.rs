use notangka::edit::{apply, Op};
use notangka::model::*;
use notangka::{format, parse};

fn run(src: &str, ops: &[&str]) -> (String, Option<notangka::edit::Caret>) {
    let mut d = parse(src).doc;
    let mut caret = None;
    for o in ops {
        let op: Op = serde_json::from_str(o).unwrap();
        caret = apply(&mut d, &op).unwrap();
    }
    (format(&d), caret)
}

fn music(f: &str) -> String {
    f.lines()
        .skip_while(|l| !l.starts_with("suara"))
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string()
}

#[test]
fn ketik_lagu_dari_kosong() {
    let (f, c) = run(
        "",
        &[
            r#"{"op":"insert","voice":0,"at":0,"digit":"1","dur":960}"#,
            r#"{"op":"insert","voice":0,"at":1,"digit":"2","dur":960}"#,
            r#"{"op":"insertBar","voice":0,"at":2,"kind":"single"}"#,
        ],
    );
    assert_eq!(music(&f), "1 2 |");
    assert_eq!(c.unwrap().index, 3);
}

#[test]
fn ketik_setengah_ketuk_mengisi_titik() {
    let (f, c) = run(
        "suara S { 1 }",
        &[r#"{"op":"insert","voice":0,"at":1,"digit":"5","dur":480}"#],
    );
    assert_eq!(music(&f), "1 [5 .]");
    assert_eq!(c.unwrap().index, 2);
    let (f, _) = run(
        "suara S { 1 }",
        &[
            r#"{"op":"insert","voice":0,"at":1,"digit":"5","dur":480}"#,
            r#"{"op":"insert","voice":0,"at":2,"digit":"6","dur":480}"#,
            r#"{"op":"insert","voice":0,"at":3,"digit":"7","dur":480}"#,
        ],
    );
    assert_eq!(music(&f), "1 [5 6] [7 .]");
}

#[test]
fn ganti_nada_oktaf_kres() {
    let (f, c) = run(
        "suara S { 1 2 3 }",
        &[
            r#"{"op":"setPitch","voice":0,"at":1,"digit":"5"}"#,
            r#"{"op":"octave","voice":0,"at":1,"delta":-1}"#,
            r#"{"op":"accidental","voice":0,"at":2,"value":1}"#,
        ],
    );
    assert_eq!(music(&f), "1 5, 3#");
    assert_eq!(c.unwrap().index, 2);
}

#[test]
fn ubah_panjang_tanpa_menggeser() {
    let (f, _) = run(
        "suara S { 1 2 | 3 }",
        &[r#"{"op":"setDuration","voice":0,"at":0,"dur":480}"#],
    );
    assert_eq!(music(&f), "[1 .] 2 | 3");
    let (f, _) = run(
        "suara S { [1 2] 3 }",
        &[r#"{"op":"setDuration","voice":0,"at":0,"dur":960}"#],
    );
    assert_eq!(music(&f), "1 3");
}

#[test]
fn hapus_not() {
    let (f, _) = run(
        "suara S { 1 2 3 }",
        &[r#"{"op":"delete","voice":0,"at":1}"#],
    );
    assert_eq!(music(&f), "1 3");
    let (f, _) = run(
        "suara S { [1 2] 3 }",
        &[r#"{"op":"delete","voice":0,"at":1}"#],
    );
    assert_eq!(music(&f), "1 3");
}

#[test]
fn legato_napas_label() {
    let (f, _) = run(
        "suara S { 1 2 3 }",
        &[
            r#"{"op":"toggleSlur","voice":0,"at":0}"#,
            r#"{"op":"toggleBreath","voice":0,"at":1}"#,
            r#"{"op":"setLabel","voice":0,"at":2,"text":"Fine"}"#,
        ],
    );
    assert_eq!(music(&f), "(1 2) ; @Fine 3");
    let (f, _) = run(
        "suara S { (1 2) 3 }",
        &[r#"{"op":"toggleSlur","voice":0,"at":0}"#],
    );
    assert_eq!(music(&f), "1 2 3");
}

#[test]
fn lirik_dan_header() {
    let (f, _) = run(
        "suara S { 1 2 3 }",
        &[
            r#"{"op":"setSyllable","verse":1,"slot":0,"text":"Ha-"}"#,
            r#"{"op":"setSyllable","verse":1,"slot":1,"text":"le-lu"}"#,
            r#"{"op":"setMeta","key":"judul","value":"Lagu Baru"}"#,
        ],
    );
    assert!(f.contains("judul = \"Lagu Baru\""));
    assert!(f.contains("bait 1 {\n  Ha-le-lu\n}"), "{}", f);
}

#[test]
fn tambah_suara_mengikuti_irama() {
    let (f, c) = run(
        "suara S { 1 [2 3] | 4 }",
        &[r#"{"op":"addVoice","name":"A"}"#],
    );
    assert!(f.contains("suara A {\n  0 [0 0] | 0\n}"), "{}", f);
    assert_eq!(c.unwrap().voice, 1);
}

#[test]
fn hasil_edit_selalu_bisa_dibaca_lagi() {
    let src = include_str!("../examples-nada/muliakan.nada");
    let ops = [
        r#"{"op":"setDuration","voice":0,"at":3,"dur":240}"#,
        r#"{"op":"insert","voice":2,"at":5,"digit":"6","dur":320}"#,
        r#"{"op":"delete","voice":1,"at":0}"#,
        r#"{"op":"toggleBreak","voice":0,"at":10}"#,
    ];
    let (f, _) = run(src, &ops);
    let p = parse(&f);
    assert!(!p.has_errors(), "{}\n{:?}", f, p.diagnostics);
    assert_eq!(format(&p.doc), f);
    let _ = Pitch::Rest;
}
