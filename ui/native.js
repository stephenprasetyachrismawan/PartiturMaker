// Berkas: lewat Tauri bila berjalan sebagai aplikasi desktop, lewat browser bila tidak.
const tauri = window.__TAURI__ && window.__TAURI__.core;
export const isDesktop = !!tauri;

export async function openFile() {
  if (tauri) return tauri.invoke('open_file');
  return new Promise((resolve) => {
    const inp = document.createElement('input');
    inp.type = 'file';
    inp.accept = '.nada,.txt';
    inp.onchange = async () => {
      const f = inp.files[0];
      resolve(f ? { path: f.name, content: await f.text() } : null);
    };
    inp.click();
  });
}

/** Simpan teks. `path` kosong = tanya lokasi. Mengembalikan path atau null. */
export async function saveText(path, content, suggested) {
  if (tauri) {
    const target = path || (await tauri.invoke('save_dialog', { suggested, ext: 'nada' }));
    if (!target) return null;
    await tauri.invoke('write_text', { path: target, content });
    return target;
  }
  download(new Blob([content], { type: 'text/plain' }), suggested);
  return suggested;
}

/** Simpan berkas biner (base64). Mengembalikan path atau null. */
export async function saveBinary(base64, suggested, ext) {
  if (tauri) {
    const target = await tauri.invoke('save_dialog', { suggested, ext });
    if (!target) return null;
    await tauri.invoke('write_base64', { path: target, data: base64 });
    return target;
  }
  const bin = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
  download(new Blob([bin]), suggested);
  return suggested;
}

export async function pendingFile() {
  if (!tauri) return null;
  return tauri.invoke('opened_with');
}

function download(blob, name) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
}
