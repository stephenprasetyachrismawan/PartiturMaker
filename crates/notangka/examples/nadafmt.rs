//! Rapikan berkas .nada: `cargo run --example nadafmt -- lagu.nada`
fn main() {
    let path = std::env::args().nth(1).expect("berkas .nada");
    let src = std::fs::read_to_string(&path).unwrap();
    match notangka::format_source(&src) {
        Ok(s) => print!("{}", s),
        Err(d) => {
            for x in d {
                eprintln!("{}:{}:{} {}", path, x.line, x.col, x.message);
            }
            std::process::exit(1);
        }
    }
}
