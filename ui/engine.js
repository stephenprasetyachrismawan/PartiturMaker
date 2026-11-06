// Memanggil inti Rust (notangka.wasm). Semua permintaan dan jawaban berupa JSON.
let wasm = null;
const enc = new TextEncoder();
const dec = new TextDecoder();

export async function loadEngine(url = 'notangka.wasm') {
  const res = await fetch(url);
  const bytes = await res.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  wasm = instance.exports;
}

function readResult(ptr) {
  const len = new DataView(wasm.memory.buffer).getUint32(ptr, true);
  const text = dec.decode(new Uint8Array(wasm.memory.buffer, ptr + 4, len));
  wasm.pm_free(ptr, len + 4);
  return text;
}

export function call(req) {
  const bytes = enc.encode(JSON.stringify(req));
  const ptr = wasm.pm_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const out = wasm.pm_call(ptr, bytes.length);
  wasm.pm_free(ptr, bytes.length);
  return JSON.parse(readResult(out));
}

export function examples() {
  return JSON.parse(readResult(wasm.pm_examples()));
}
