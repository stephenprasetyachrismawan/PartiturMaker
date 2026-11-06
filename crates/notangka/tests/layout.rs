use notangka::layout::{layout, Mode, Options, Prim};
use notangka::{api, parse, EXAMPLES};

#[test]
fn semua_contoh_tergambar_buku_dan_tayang() {
    for (_, src) in EXAMPLES {
        let d = parse(src).doc;
        for mode in [Mode::Buku, Mode::Tayang] {
            let pages = layout(
                &d,
                &Options {
                    mode,
                    ..Default::default()
                },
            );
            assert!(!pages.is_empty());
            let svg = notangka::svg::render(&pages[0], true);
            assert!(svg.starts_with("<svg"));
        }
    }
}

#[test]
fn garis_harga_di_atas_angka() {
    // Satu not setengah ketuk: garis harus di atas garis dasar angka.
    let d = parse("suara S { [1 2] }").doc;
    let page = &layout(&d, &Options::default())[0];
    let digit_y = page
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Text { s, y, .. } if s == "1" => Some(*y),
            _ => None,
        })
        .unwrap();
    let beam = page
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Rect { y, w, h, .. } if *w > 5.0 && *h < 2.0 => Some(*y),
            _ => None,
        })
        .unwrap();
    assert!(
        beam < digit_y - 8.0,
        "garis harga {} harus di atas angka {}",
        beam,
        digit_y
    );
}

#[test]
fn legato_di_bawah_angka() {
    let d = parse("suara S { (1 2) }").doc;
    let page = &layout(&d, &Options::default())[0];
    let digit_y = page
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Text { s, y, .. } if s == "1" => Some(*y),
            _ => None,
        })
        .unwrap();
    let slur_y = page
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Path { d, stroke: None } => match d[0] {
                notangka::layout::Seg::M(_, y) => Some(y),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert!(
        slur_y > digit_y,
        "legato {} harus di bawah angka {}",
        slur_y,
        digit_y
    );
}

#[test]
fn lagu_panjang_banyak_halaman() {
    let bar = "1 2 3 4 | 5 6 7 1' | ".repeat(60);
    let lyr = "la ".repeat(480);
    let src = format!(
        "ketuk = 4\nsuara S {{ {} }}\nbait 1 {{ {} }}\nbait 2 {{ {} }}",
        bar, lyr, lyr
    );
    let pages = layout(
        &parse(&src).doc,
        &Options {
            page: "A5".into(),
            ..Default::default()
        },
    );
    assert!(pages.len() > 1);
}

#[test]
fn pdf_sah() {
    let out = api::handle(&serde_json::json!({ "fn": "pdf", "src": EXAMPLES[0].1 }).to_string());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let b64 = v["pdf"].as_str().unwrap();
    assert!(b64.starts_with("JVBERi0")); // "%PDF-"
}

#[test]
fn api_render_dan_apply() {
    let out = api::handle(&serde_json::json!({ "fn": "render", "src": "suara S { 1 2 }", "opts": { "editing": true } }).to_string());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["pages"][0].as_str().unwrap().contains("data-k=\"note\""));
    assert_eq!(v["doc"]["voices"][0]["items"].as_array().unwrap().len(), 2);
    let out = api::handle(&serde_json::json!({ "fn": "apply", "src": "suara S { 1 2 }", "op": { "op": "octave", "voice": 0, "at": 1, "delta": 1 } }).to_string());
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["src"].as_str().unwrap().contains("1 2'"), "{}", v);
    let out = api::handle(&serde_json::json!({ "fn": "apply", "src": "suara S { 1 x }", "op": { "op": "delete", "voice": 0, "at": 0 } }).to_string());
    assert!(out.contains("error"));
}

#[test]
fn timeline_ikatan_dan_titik() {
    let v = api::timeline("nada = C\nsuara S { 1 . (5 5) 0 3 }");
    let notes = v["notes"].as_array().unwrap();
    let got: Vec<(i64, f64, f64)> = notes
        .iter()
        .map(|n| {
            (
                n["midi"].as_i64().unwrap(),
                n["start"].as_f64().unwrap(),
                n["dur"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(got, vec![(60, 0.0, 2.0), (67, 2.0, 2.0), (64, 5.0, 1.0)]);
}
