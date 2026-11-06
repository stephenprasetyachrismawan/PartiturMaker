// Bangun inti Rust ke WebAssembly dan salin ke ui/notangka.wasm.
// Pakai: node scripts/build-wasm.mjs
import { execSync } from 'node:child_process';
import { copyFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
execSync('cargo build -p notangka-wasm --release --target wasm32-unknown-unknown', { cwd: root, stdio: 'inherit' });
const src = join(root, 'target', 'wasm32-unknown-unknown', 'release', 'notangka_wasm.wasm');
const dst = join(root, 'ui', 'notangka.wasm');
copyFileSync(src, dst);
console.log(`ui/notangka.wasm ${(statSync(dst).size / 1024).toFixed(0)} KB`);
