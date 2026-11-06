//! `cargo run --example render -- lagu.nada keluar/ [buku|tayang]` -> SVG tiap halaman + PDF.
use notangka::layout::{layout, Mode, Options};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let out = std::path::Path::new(&a[2]);
    std::fs::create_dir_all(out).unwrap();
    let p = notangka::parse(&src);
    for d in &p.diagnostics {
        eprintln!("{}:{} {:?} {}", d.line, d.col, d.severity, d.message);
    }
    let mode = if a.get(3).map(|s| s.as_str()) == Some("tayang") {
        Mode::Tayang
    } else {
        Mode::Buku
    };
    let editing = a.get(4).map(|s| s == "edit").unwrap_or(false);
    let opts = Options {
        mode,
        editing,
        ..Default::default()
    };
    let pages = layout(&p.doc, &opts);
    for (i, pg) in pages.iter().enumerate() {
        std::fs::write(
            out.join(format!("hal-{}.svg", i + 1)),
            notangka::svg::render(pg, editing),
        )
        .unwrap();
    }
    std::fs::write(out.join("lagu.pdf"), notangka::pdf::render(&pages, "uji")).unwrap();
    println!("{} halaman", pages.len());
}
