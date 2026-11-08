//! Jembatan WASM sederhana tanpa wasm-bindgen.
//!
//! JS menulis permintaan JSON ke memori (`pm_alloc`), memanggil `pm_call`,
//! lalu membaca hasil: 4 byte panjang (little endian) + isi UTF-8.
//! Setelah dibaca, hasil dibebaskan dengan `pm_free(ptr, 4 + panjang)`.

use std::alloc::{alloc, dealloc, Layout};

#[no_mangle]
pub extern "C" fn pm_alloc(len: usize) -> *mut u8 {
    unsafe { alloc(Layout::from_size_align(len.max(1), 1).unwrap()) }
}

/// # Safety
/// `ptr` harus berasal dari `pm_alloc`/`pm_call` dengan panjang yang sama.
#[no_mangle]
pub unsafe extern "C" fn pm_free(ptr: *mut u8, len: usize) {
    dealloc(ptr, Layout::from_size_align(len.max(1), 1).unwrap());
}

/// # Safety
/// `ptr..ptr+len` harus berisi UTF-8 yang ditulis lewat `pm_alloc`.
#[no_mangle]
pub unsafe extern "C" fn pm_call(ptr: *const u8, len: usize) -> *mut u8 {
    let input = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap_or("{}");
    let out = notangka::api::handle(input);
    let bytes = out.as_bytes();
    let total = bytes.len() + 4;
    let p = pm_alloc(total);
    std::ptr::copy_nonoverlapping((bytes.len() as u32).to_le_bytes().as_ptr(), p, 4);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p.add(4), bytes.len());
    p
}

/// Contoh lagu bawaan sebagai JSON: [[nama, teks], ...].
#[no_mangle]
pub extern "C" fn pm_examples() -> *mut u8 {
    let list: Vec<(&str, &str)> = notangka::EXAMPLES.to_vec();
    let json = serde_json_lite(&list);
    let bytes = json.as_bytes();
    let total = bytes.len() + 4;
    let p = pm_alloc(total);
    unsafe {
        std::ptr::copy_nonoverlapping((bytes.len() as u32).to_le_bytes().as_ptr(), p, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p.add(4), bytes.len());
    }
    p
}

fn serde_json_lite(list: &[(&str, &str)]) -> String {
    let esc = |s: &str| {
        let mut o = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => {}
                '\t' => o.push_str("\\t"),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    };
    let items: Vec<String> = list
        .iter()
        .map(|(a, b)| format!("[{},{}]", esc(a), esc(b)))
        .collect();
    format!("[{}]", items.join(","))
}
