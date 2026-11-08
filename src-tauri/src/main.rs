//! Aplikasi desktop PartiturMaker. Antarmuka ada di folder `ui/` dan memakai
//! inti Rust lewat WebAssembly; sisi ini hanya mengurus berkas.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
struct Opened {
    path: String,
    content: String,
}

fn read(path: PathBuf) -> Result<Opened, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Gagal membaca berkas: {}", e))?;
    Ok(Opened {
        path: path.to_string_lossy().into_owned(),
        content,
    })
}

/// Berkas yang dibuka dengan klik dua kali (argumen program).
#[tauri::command]
fn opened_with() -> Option<Opened> {
    std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.extension().map(|e| e == "nada").unwrap_or(false) && p.exists())
        .and_then(|p| read(p).ok())
}

#[tauri::command]
async fn open_file(app: tauri::AppHandle) -> Result<Option<Opened>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("Buka partitur")
        .add_filter("Partitur Nada", &["nada"])
        .add_filter("Teks", &["txt"])
        .blocking_pick_file();
    match picked {
        Some(f) => {
            let p = f.into_path().map_err(|e| e.to_string())?;
            read(p).map(Some)
        }
        None => Ok(None),
    }
}

#[tauri::command]
async fn save_dialog(
    app: tauri::AppHandle,
    suggested: String,
    ext: String,
) -> Result<Option<String>, String> {
    let label = match ext.as_str() {
        "nada" => "Partitur Nada",
        "pdf" => "PDF",
        "png" => "Gambar PNG",
        "svg" => "Gambar SVG",
        _ => "Berkas",
    };
    let picked = app
        .dialog()
        .file()
        .set_title("Simpan")
        .set_file_name(&suggested)
        .add_filter(label, &[ext.as_str()])
        .blocking_save_file();
    match picked {
        Some(f) => {
            let mut p = f.into_path().map_err(|e| e.to_string())?;
            if p.extension().is_none() {
                p.set_extension(&ext);
            }
            Ok(Some(p.to_string_lossy().into_owned()))
        }
        None => Ok(None),
    }
}

#[tauri::command]
fn write_text(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("Gagal menyimpan: {}", e))
}

#[tauri::command]
fn write_base64(path: String, data: String) -> Result<(), String> {
    let bytes = decode_base64(&data).ok_or("Data rusak")?;
    std::fs::write(&path, bytes).map_err(|e| format!("Gagal menyimpan: {}", e))
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let mut v = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            v |= if c == b'=' { 0 } else { val(c)? } << (18 - 6 * i);
        }
        out.push((v >> 16) as u8);
        if pad < 2 {
            out.push((v >> 8) as u8);
        }
        if pad < 1 {
            out.push(v as u8);
        }
    }
    Some(out)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            opened_with,
            open_file,
            save_dialog,
            write_text,
            write_base64
        ])
        .run(tauri::generate_context!())
        .expect("PartiturMaker gagal dijalankan");
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_bolak_balik() {
        assert_eq!(super::decode_base64("SGFsbw==").unwrap(), b"Halo");
        assert_eq!(super::decode_base64("SGFsbG8h").unwrap(), b"Hallo!");
    }
}
