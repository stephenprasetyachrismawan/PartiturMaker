// Editor PartiturMaker: menulis langsung di halaman partitur.
// Setiap suntingan visual menjadi operasi pada inti Rust, yang menulis ulang kode Nada.
import { loadEngine, call, examples } from './engine.js';
import { openFile, saveText, saveBinary, pendingFile, isDesktop } from './native.js';
import { Player } from './player.js';

const $ = (s) => document.querySelector(s);
const $$ = (s) => Array.from(document.querySelectorAll(s));
const BEAT = 960;
const PT = 96 / 72; // 1pt dalam px CSS
const VOICE_NAMES = { S: 'Sopran', A: 'Alto', T: 'Tenor', B: 'Bas' };

const saved = load('pm.settings') || {};
const state = {
  src: '',
  filePath: null,
  dirty: false,
  history: [],
  future: [],
  mode: 'buku',
  page: saved.page || 'A4',
  noteSize: saved.noteSize || 15,
  lyricSize: saved.lyricSize || 11.5,
  zoom: saved.zoom || 'fit',
  size: [595, 842],
  codeOpen: !!saved.codeOpen,
  dur: BEAT,
  doc: null,
  diagnostics: [],
  sel: null,
};
const player = new Player();
const paper = $('#paper');
const stage = $('#stage');
const code = $('#code');

function load(k) { try { return JSON.parse(localStorage.getItem(k)); } catch (e) { return null; } }
function store(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) { /* tidak tersedia */ } }
function saveSettings() {
  store('pm.settings', { page: state.page, noteSize: state.noteSize, lyricSize: state.lyricSize, zoom: state.zoom, codeOpen: state.codeOpen });
}

// ---------------------------------------------------------------------------
// Menggambar

function opts(editing = true) {
  return { mode: state.mode, page: state.page, noteSize: state.noteSize, lyricSize: state.lyricSize, editing };
}

function render() {
  const r = call({ fn: 'render', src: state.src, opts: opts(state.mode === 'buku') });
  paper.innerHTML = r.pages.join('');
  if (r.size) state.size = r.size;
  applyZoom();
  state.doc = r.doc;
  state.diagnostics = r.diagnostics;
  syncCode();
  syncSide();
  drawSelection();
  showDiagnostics();
}

function setSrc(src, { push = true, keepCode = false } = {}) {
  if (src === state.src) return;
  if (push) {
    state.history.push(state.src);
    if (state.history.length > 300) state.history.shift();
    state.future = [];
  }
  state.src = src;
  setDirty(true);
  store('pm.draft', { src, filePath: state.filePath });
  if (keepCode) codeTouched = true;
  render();
}

function setDirty(d) {
  state.dirty = d;
  const name = state.filePath ? state.filePath.split(/[\\/]/).pop() : (state.doc && state.doc.meta.judul) || 'Lagu baru';
  $('#file-name').textContent = name;
  $('#save-state').textContent = d ? 'Belum disimpan' : 'Tersimpan';
  document.title = name + ' - PartiturMaker';
}

function op(o) {
  const r = call({ fn: 'apply', src: state.src, op: o });
  if (r.error) {
    say(r.error, true);
    return null;
  }
  setSrc(r.src);
  return r.caret || null;
}

// ---------------------------------------------------------------------------
// Pilihan dan kursor

function items(v) {
  const voice = state.doc && state.doc.voices[v];
  return voice ? voice.items : [];
}

function selectAt(voice, index, prefer) {
  const list = items(voice);
  if (index >= list.length) {
    state.sel = { k: 'insert', voice, index: list.length };
  } else if (prefer === 'insert') {
    state.sel = { k: 'insert', voice, index };
  } else {
    const it = list[index];
    state.sel = { k: it.k === 'b' ? 'bar' : it.k === 'br' ? 'insert' : 'note', voice, index };
  }
  drawSelection();
}

function drawSelection() {
  paper.querySelectorAll('.hit.sel').forEach((e) => e.classList.remove('sel'));
  paper.querySelectorAll('.caret').forEach((e) => e.remove());
  const s = state.sel;
  if (!s) { describe(); return; }
  let el = null;
  if (s.k === 'note' || s.k === 'bar') {
    el = paper.querySelector(`.hit[data-k="${s.k === 'bar' ? 'bar' : 'note'}"][data-v="${s.voice}"][data-i="${s.index}"]`);
    if (el) el.classList.add('sel');
  } else if (s.k === 'insert') {
    const target = paper.querySelector(`.hit[data-v="${s.voice}"][data-i="${s.index}"]`);
    if (target && target.dataset.k !== 'end') {
      const x = parseFloat(target.getAttribute('x'));
      const y = parseFloat(target.getAttribute('y'));
      const h = parseFloat(target.getAttribute('height'));
      const c = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      c.setAttribute('class', 'caret');
      c.setAttribute('x', x - 0.8);
      c.setAttribute('y', y);
      c.setAttribute('width', 1.6);
      c.setAttribute('height', h);
      target.parentNode.appendChild(c);
      el = target;
    } else {
      el = paper.querySelector(`.hit[data-k="end"][data-v="${s.voice}"]`);
      if (el) el.classList.add('sel');
    }
  } else if (s.k === 'syl') {
    el = paper.querySelector(`.hit[data-k="syl"][data-verse="${s.verse}"][data-slot="${s.slot}"]`);
    if (el) el.classList.add('sel');
  }
  if (el && el.scrollIntoView) el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  describe();
}

function beatsLabel(t) {
  const b = t / BEAT;
  return Number.isInteger(b) ? String(b) : b.toFixed(2).replace(/0+$/, '').replace('.', ',');
}

function durLabel(d) {
  return { 960: '1 ketuk', 480: '½ ketuk', 240: '¼ ketuk', 120: '⅛ ketuk', 320: 'triol', 160: 'triol ½' }[d] || beatsLabel(d) + ' ketuk';
}

function describe() {
  const s = state.sel;
  if (state.mode === 'tayang') return say('Tayangan per bait hanya untuk dilihat dan diekspor. Sunting di Halaman buku.');
  if (!s) return say('Klik not untuk memilih. Ketik 1 sampai 7 untuk menulis.');
  const voice = state.doc && state.doc.voices[s.voice];
  const vname = voice ? VOICE_NAMES[voice.name] : 'Sopran';
  if (s.k === 'note') {
    const it = items(s.voice)[s.index];
    if (!it) return;
    const what = it.k === 'h' ? 'titik perpanjangan' : it.k === 'r' ? 'tanda diam' : 'not';
    say(`${vname}, ketuk ke-${beatsLabel(it.t + BEAT).replace(/,.*/, '')}: ${what}, ${durLabel(it.dur)}. Ketik angka untuk mengganti, panah atas/bawah untuk oktaf.`);
  } else if (s.k === 'bar') {
    say(`${vname}: garis birama. Ketik angka untuk menyisipkan not sesudahnya, Delete untuk menghapus.`);
  } else if (s.k === 'insert') {
    say(`${vname}: menulis not ${durLabel(state.dur)}. Ketik 1 sampai 7, 0 untuk diam, titik untuk menahan, | untuk garis birama.`);
  } else if (s.k === 'syl') {
    say(`Bait ${s.verse}: spasi untuk suku kata berikutnya, tanda hubung untuk menyambung kata, Enter untuk selesai.`);
  }
}

function say(msg, error = false) {
  const el = $('#status');
  el.textContent = msg;
  el.classList.toggle('error', error);
}

function move(delta) {
  const s = state.sel;
  if (!s || !['note', 'bar', 'insert'].includes(s.k)) return;
  const list = items(s.voice);
  let i = s.index + delta;
  while (i >= 0 && i < list.length && list[i].k === 'br') i += delta;
  i = Math.max(0, Math.min(list.length, i));
  selectAt(s.voice, i);
}

function changeVoice(delta) {
  const s = state.sel;
  if (!s || !state.doc || state.doc.voices.length < 2) return;
  const nv = Math.max(0, Math.min(state.doc.voices.length - 1, s.voice + delta));
  if (nv === s.voice) return;
  const t = s.index < items(s.voice).length ? items(s.voice)[s.index].t : Infinity;
  const list = items(nv);
  let idx = list.findIndex((it) => it.k !== 'b' && it.k !== 'br' && it.t >= t);
  if (idx < 0) idx = list.length;
  selectAt(nv, idx);
}

// ---------------------------------------------------------------------------
// Menulis not

function typeDigit(ch) {
  if (state.mode !== 'buku') return;
  let s = state.sel;
  if (!s || s.k === 'syl') {
    const v = 0;
    s = state.sel = { k: 'insert', voice: v, index: items(v).length };
  }
  if (s.k === 'note') {
    const caret = op({ op: 'setPitch', voice: s.voice, at: s.index, digit: ch });
    if (caret) selectAt(caret.voice, caret.index);
    return;
  }
  const at = s.k === 'bar' ? s.index + 1 : s.index;
  const caret = op({ op: 'insert', voice: s.voice, at, digit: ch, dur: state.dur });
  if (!caret) return;
  // Bila sesudahnya ada titik pelengkap di tengah ketuk, pilih titik itu agar langsung ditimpa.
  const next = items(caret.voice)[caret.index];
  if (next && next.k === 'h' && next.t % BEAT !== 0) selectAt(caret.voice, caret.index);
  else selectAt(caret.voice, caret.index, 'insert');
}

function needNote() {
  const s = state.sel;
  if (s && s.k === 'note') return s;
  say('Pilih sebuah not dulu dengan mengekliknya.', true);
  return null;
}

function act(name) {
  if (state.mode !== 'buku') return;
  const s = state.sel;
  switch (name) {
    case 'hold': return typeDigit('.');
    case 'rest': return typeDigit('0');
    case 'up':
    case 'down': {
      const n = needNote();
      if (n) { op({ op: 'octave', voice: n.voice, at: n.index, delta: name === 'up' ? 1 : -1 }); selectAt(n.voice, n.index); }
      return;
    }
    case 'sharp':
    case 'flat': {
      const n = needNote();
      if (n) { op({ op: 'accidental', voice: n.voice, at: n.index, value: name === 'sharp' ? 1 : -1 }); selectAt(n.voice, n.index); }
      return;
    }
    case 'slur':
    case 'breath':
    case 'fermata': {
      const n = needNote();
      if (!n) return;
      const o = { slur: 'toggleSlur', breath: 'toggleBreath', fermata: 'toggleFermata' }[name];
      if (op({ op: o, voice: n.voice, at: n.index })) selectAt(n.voice, n.index);
      return;
    }
    case 'label': {
      const n = needNote();
      if (!n) return;
      const cur = items(n.voice)[n.index].label || '';
      const el = paper.querySelector(`.hit[data-k="note"][data-v="${n.voice}"][data-i="${n.index}"]`);
      openInline(el, cur, (text) => {
        op({ op: 'setLabel', voice: n.voice, at: n.index, text });
        selectAt(n.voice, n.index);
      }, { above: true, placeholder: 'Refrein' });
      return;
    }
    case 'break': {
      if (!s || !['note', 'bar', 'insert'].includes(s.k)) return say('Pilih tempat pindah baris dulu.', true);
      const at = s.k === 'bar' ? s.index + 1 : s.index;
      const caret = op({ op: 'toggleBreak', voice: s.voice, at });
      if (caret) selectAt(caret.voice, caret.index);
      return;
    }
    case 'delete': return del(false);
  }
}

function del(backward) {
  const s = state.sel;
  if (!s || state.mode !== 'buku') return;
  if (s.k === 'insert') {
    if (!backward || s.index === 0) return;
    const caret = op({ op: 'delete', voice: s.voice, at: s.index - 1 });
    if (caret) selectAt(caret.voice, caret.index, 'insert');
    return;
  }
  if (s.k === 'note' || s.k === 'bar') {
    const caret = op({ op: 'delete', voice: s.voice, at: s.index });
    if (caret) selectAt(caret.voice, Math.max(0, caret.index - (backward ? 1 : 0)), backward ? 'insert' : undefined);
  }
}

function bar(kind) {
  const s = state.sel;
  if (state.mode !== 'buku') return;
  if (s && s.k === 'bar') {
    op({ op: 'setBar', voice: s.voice, at: s.index, kind });
    selectAt(s.voice, s.index);
    return;
  }
  const voice = s ? s.voice : 0;
  const at = !s ? items(0).length : s.k === 'note' ? s.index + 1 : s.index;
  const caret = op({ op: 'insertBar', voice, at, kind });
  if (caret) selectAt(caret.voice, caret.index, 'insert');
}

function setDur(d) {
  state.dur = d;
  $$('.tool[data-dur]').forEach((b) => b.setAttribute('aria-pressed', String(Number(b.dataset.dur) === d)));
  const s = state.sel;
  if (s && s.k === 'note') {
    op({ op: 'setDuration', voice: s.voice, at: s.index, dur: d });
    selectAt(s.voice, s.index);
  } else describe();
}

function stepDur(dir) {
  const ladder = [240, 480, 960];
  const s = state.sel;
  const cur = s && s.k === 'note' ? items(s.voice)[s.index].dur : state.dur;
  let i = ladder.indexOf(cur);
  if (i < 0) i = dir < 0 ? 1 : 2;
  else i = Math.max(0, Math.min(2, i + dir));
  setDur(ladder[i]);
}

// ---------------------------------------------------------------------------
// Penyunting kecil di atas partitur (lirik, judul, label)

const inline = $('#inline-edit');
const area = $('#inline-area');
let inlineDone = null;

function placeOver(el, box, { above = false } = {}) {
  const r = el.getBoundingClientRect();
  const sr = stage.getBoundingClientRect();
  box.style.left = r.left - sr.left + stage.scrollLeft + 'px';
  box.style.top = (above ? r.top - r.height * 0.9 : r.top) - sr.top + stage.scrollTop + 'px';
  box.style.height = Math.max(22, r.height) + 'px';
  box.style.minWidth = Math.max(60, r.width + 24) + 'px';
}

function openInline(el, value, done, o = {}) {
  if (!el) return;
  closeInline(false);
  placeOver(el, inline, o);
  inline.style.fontSize = (o.size || state.lyricSize) * PT * zoomValue() + 'px';
  inline.value = value || '';
  inline.placeholder = o.placeholder || '';
  inline.hidden = false;
  inlineDone = { done, keys: o.keys };
  inline.focus();
  inline.select();
}

function closeInline(commit) {
  if (inline.hidden) return;
  const d = inlineDone;
  inline.hidden = true;
  inlineDone = null;
  if (commit && d) d.done(inline.value);
  stage.focus({ preventScroll: true });
}

inline.addEventListener('keydown', (e) => {
  const d = inlineDone;
  if (!d) return;
  if (d.keys && d.keys(e)) return;
  if (e.key === 'Enter') { e.preventDefault(); closeInline(true); }
  else if (e.key === 'Escape') { e.preventDefault(); closeInline(false); }
});
inline.addEventListener('blur', () => setTimeout(() => { if (document.activeElement !== inline) closeInline(true); }, 0));

function editSyllable(verse, slot) {
  const v = state.doc.verses.find((x) => x.number === verse);
  const cur = (v && v.syllables[slot]) || '';
  let el = paper.querySelector(`.hit[data-k="syl"][data-verse="${verse}"][data-slot="${slot}"]`);
  if (!el) {
    // Bait belum ada: taruh di bawah not pemilik slot.
    const ii = items(0).findIndex((it) => it.slot === slot);
    el = paper.querySelector(`.hit[data-k="note"][data-v="0"][data-i="${ii}"]`);
  }
  state.sel = { k: 'syl', verse, slot };
  drawSelection();
  const go = (text, next) => {
    inlineDone = null;
    inline.hidden = true;
    if (text !== cur) op({ op: 'setSyllable', verse, slot, text });
    const target = slot + next;
    if (next !== 0 && target >= 0 && target < state.doc.slots) editSyllable(verse, target);
    else { state.sel = null; drawSelection(); stage.focus({ preventScroll: true }); }
  };
  openInline(el, cur, (text) => go(text, 0), {
    placeholder: 'suku kata',
    keys: (e) => {
      const val = inline.value;
      if (e.key === ' ' || (e.key === 'Tab' && !e.shiftKey)) { e.preventDefault(); go(val, 1); return true; }
      if (e.key === '-' && val && !val.endsWith('-')) { e.preventDefault(); go(val + '-', 1); return true; }
      if (e.key === 'Tab' && e.shiftKey) { e.preventDefault(); go(val, -1); return true; }
      if (e.key === 'Backspace' && val === '' && slot > 0) { e.preventDefault(); go(val, -1); return true; }
      return false;
    },
  });
}

const META_LABEL = { judul: 'Judul lagu', nomor: 'KJ 1', ayat: 'Mazmur 100', tempo: 'Khidmat', syair: 'Nama penulis', lagu: 'Nama komponis', terjemahan: 'Penerjemah', aransemen: 'Penata', catatan: 'Catatan' };

function editMeta(key, el) {
  if (key === 'nada') { const f = $('[data-meta="nada"]'); f.focus(); return; }
  const cur = state.doc.meta[key] || '';
  openInline(el, cur, (text) => {
    if (text !== cur) op({ op: 'setMeta', key, value: text });
  }, { size: key === 'judul' ? state.noteSize * 1.3 : state.lyricSize, placeholder: META_LABEL[key] || '' });
}

function editText(number, el) {
  const t = state.doc.texts.find((x) => x.number === number);
  closeInline(false);
  placeOver(el, area);
  area.style.fontSize = state.lyricSize * PT * zoomValue() + 'px';
  area.value = t ? t.text : '';
  area.hidden = false;
  area.focus();
  const finish = (commit) => {
    if (area.hidden) return;
    area.hidden = true;
    if (commit && (!t || area.value !== t.text)) op({ op: 'setTextVerse', number, text: area.value });
    stage.focus({ preventScroll: true });
  };
  area.onkeydown = (e) => {
    if (e.key === 'Escape') { e.preventDefault(); finish(false); }
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); finish(true); }
  };
  area.onblur = () => finish(true);
}

// ---------------------------------------------------------------------------
// Klik pada partitur

paper.addEventListener('mousedown', (e) => {
  const hit = e.target.closest('.hit');
  if (!hit) return;
  e.preventDefault();
  stage.focus({ preventScroll: true });
  const k = hit.dataset.k;
  const v = Number(hit.dataset.v);
  const i = Number(hit.dataset.i);
  if (k === 'note') selectAt(v, i);
  else if (k === 'bar') selectAt(v, i);
  else if (k === 'end') selectAt(v, items(v).length);
  else if (k === 'syl') editSyllable(Number(hit.dataset.verse), Number(hit.dataset.slot));
  else if (k === 'meta') editMeta(hit.dataset.key, hit);
  else if (k === 'text') editText(Number(hit.dataset.number), hit);
});

paper.addEventListener('dblclick', (e) => {
  const hit = e.target.closest('.hit[data-k="note"]');
  if (!hit || Number(hit.dataset.v) !== 0) return;
  const it = items(0)[Number(hit.dataset.i)];
  if (!it || it.slot == null) return;
  const verse = state.doc.verses.length ? state.doc.verses[0].number : 1;
  editSyllable(verse, it.slot);
});

// ---------------------------------------------------------------------------
// Papan ketik

function typing(el) {
  return el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
}

document.addEventListener('keydown', (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl) {
    const k = e.key.toLowerCase();
    if (k === 's') { e.preventDefault(); return save(e.shiftKey); }
    if (k === 'o') { e.preventDefault(); return openCmd(); }
    if (k === 'n') { e.preventDefault(); return newCmd(); }
    if (k === 'e') { e.preventDefault(); return exportCmd('pdf'); }
    if (k === 'p') { e.preventDefault(); return exportCmd('print'); }
    if (k === '=' || k === '+') { e.preventDefault(); return zoomBy(0.1); }
    if (k === '-') { e.preventDefault(); return zoomBy(-0.1); }
    if (k === '0') { e.preventDefault(); return zoomTo('fit'); }
    if (typing(document.activeElement)) return;
    if (k === 'z' && !e.shiftKey) { e.preventDefault(); return undo(); }
    if (k === 'y' || (k === 'z' && e.shiftKey)) { e.preventDefault(); return redo(); }
    return;
  }
  if (e.shiftKey && e.altKey && e.key.toLowerCase() === 'f') { e.preventDefault(); return formatCmd(); }
  if (e.key === 'F1') { e.preventDefault(); return $('#keys').showModal(); }
  if (typing(document.activeElement)) return;
  const k = e.key;
  if (k === '?') { e.preventDefault(); return $('#keys').showModal(); }
  if (/^[0-7]$/.test(k)) { e.preventDefault(); return typeDigit(k); }
  const map = {
    '.': () => typeDigit('.'),
    ArrowLeft: () => move(-1),
    ArrowRight: () => move(1),
    ArrowUp: () => (e.altKey ? changeVoice(-1) : act('up')),
    ArrowDown: () => (e.altKey ? changeVoice(1) : act('down')),
    "'": () => act('up'),
    ',': () => act('down'),
    '#': () => act('sharp'),
    b: () => act('flat'),
    s: () => act('slur'),
    ';': () => act('breath'),
    f: () => act('fermata'),
    l: () => act('label'),
    '|': () => bar('single'),
    '[': () => stepDur(-1),
    ']': () => stepDur(1),
    j: () => setDur(960),
    h: () => setDur(480),
    k: () => setDur(240),
    Enter: () => act('break'),
    Backspace: () => del(true),
    Delete: () => del(false),
    Escape: () => { player.stop(); state.sel = null; drawSelection(); },
    ' ': () => togglePlay(),
    Home: () => state.sel && selectAt(state.sel.voice, 0),
    End: () => state.sel && selectAt(state.sel.voice, items(state.sel.voice).length),
  };
  if (map[k]) { e.preventDefault(); map[k](); }
});

// ---------------------------------------------------------------------------
// Riwayat

function undo() {
  if (!state.history.length) return say('Tidak ada yang bisa diurungkan.');
  state.future.push(state.src);
  state.src = state.history.pop();
  setDirty(true);
  render();
}
function redo() {
  if (!state.future.length) return;
  state.history.push(state.src);
  state.src = state.future.pop();
  setDirty(true);
  render();
}

// ---------------------------------------------------------------------------
// Panel samping

let metaTimer = 0;
$$('[data-meta]').forEach((el) => {
  const commit = () => {
    const key = el.dataset.meta;
    const value = el.value;
    if ((state.doc.meta[key] || '') === value) return;
    op({ op: 'setMeta', key, value });
  };
  if (el.tagName === 'SELECT') el.addEventListener('change', commit);
  else {
    el.addEventListener('input', () => { clearTimeout(metaTimer); metaTimer = setTimeout(commit, 500); });
    el.addEventListener('change', () => { clearTimeout(metaTimer); commit(); });
  }
});

function syncSide() {
  const d = state.doc;
  if (!d) return;
  $$('[data-meta]').forEach((el) => {
    if (el === document.activeElement) return;
    const v = d.meta[el.dataset.meta] || '';
    if (el.tagName === 'SELECT' && v && !Array.from(el.options).some((o) => o.value === v)) {
      const o = document.createElement('option');
      o.value = o.textContent = v;
      el.appendChild(o);
    }
    el.value = v;
  });
  const present = d.voices.map((v) => v.name);
  $$('.chip[data-voice]').forEach((c) => c.setAttribute('aria-pressed', String(present.includes(c.dataset.voice))));
  const list = $('#verse-list');
  list.innerHTML = '';
  const rows = [
    ...d.verses.map((v) => ({ n: v.number, kind: 'bait', label: `Bait ${v.number}`, note: `${v.syllables.filter(Boolean).length} dari ${d.slots} suku kata` })),
    ...d.texts.map((t) => ({ n: t.number, kind: 'teks', label: `Bait ${t.number}`, note: 'dicetak sebagai teks' })),
  ].sort((a, b) => a.n - b.n);
  for (const r of rows) {
    const li = document.createElement('li');
    const span = document.createElement('span');
    span.innerHTML = `${r.label} <span class="quiet small"></span>`;
    span.querySelector('.quiet').textContent = r.note;
    const del = document.createElement('button');
    del.className = 'del';
    del.textContent = 'Hapus';
    del.onclick = () => {
      if (!window.confirm(`Hapus ${r.label.toLowerCase()}?`)) return;
      op(r.kind === 'bait' ? { op: 'removeVerse', verse: r.n } : { op: 'removeTextVerse', number: r.n });
    };
    li.append(span, del);
    list.appendChild(li);
  }
  if (!rows.length) {
    const li = document.createElement('li');
    li.className = 'quiet small';
    li.textContent = 'Belum ada bait. Klik dua kali sebuah not untuk mulai menulis lirik.';
    list.appendChild(li);
  }
}

$$('.chip[data-voice]').forEach((c) => c.addEventListener('click', () => {
  const name = c.dataset.voice;
  const idx = state.doc.voices.findIndex((v) => v.name === name);
  if (idx >= 0) {
    if (!window.confirm(`Hapus suara ${VOICE_NAMES[name].toLowerCase()} beserta semua notnya?`)) return;
    op({ op: 'removeVoice', voice: idx });
    state.sel = null;
    drawSelection();
  } else {
    const caret = op({ op: 'addVoice', name });
    if (caret) selectAt(caret.voice, 0);
  }
}));

$('#btn-add-verse').addEventListener('click', () => {
  op({ op: 'addVerse' });
  const v = state.doc.verses[state.doc.verses.length - 1];
  if (v && state.doc.slots) editSyllable(v.number, 0);
});
$('#btn-add-text').addEventListener('click', () => {
  op({ op: 'addTextVerse' });
  const t = state.doc.texts[state.doc.texts.length - 1];
  const el = t && paper.querySelector(`.hit[data-k="text"][data-number="${t.number}"]`);
  if (el) editText(t.number, el);
});

const selPage = $('#sel-page');
selPage.value = state.page;
selPage.addEventListener('change', () => { state.page = selPage.value; saveSettings(); render(); });
for (const [id, key, out] of [['#rng-note', 'noteSize', '#out-note'], ['#rng-lyric', 'lyricSize', '#out-lyric']]) {
  const el = $(id);
  el.value = state[key];
  $(out).textContent = state[key];
  el.addEventListener('input', () => { state[key] = Number(el.value); $(out).textContent = el.value; saveSettings(); render(); });
}

// ---------------------------------------------------------------------------
// Panel kode

let codeTimer = 0;
let codeTouched = false;
function syncCode() {
  if (document.activeElement === code && codeTouched) return;
  if (code.value !== state.src) {
    const top = code.scrollTop;
    code.value = state.src;
    code.scrollTop = top;
  }
  drawGutter();
}

function drawGutter() {
  const n = code.value.split('\n').length;
  const errs = new Map();
  for (const d of state.diagnostics) if (!errs.has(d.line) || d.severity === 'Error') errs.set(d.line, d.severity);
  let html = '';
  for (let i = 1; i <= n; i++) {
    const sev = errs.get(i);
    html += sev ? `<span class="${sev === 'Error' ? 'err' : 'warn'}">${i}</span>\n` : i + '\n';
  }
  $('#gutter').innerHTML = html;
  $('#gutter').scrollTop = code.scrollTop;
}

code.addEventListener('input', () => {
  codeTouched = true;
  clearTimeout(codeTimer);
  codeTimer = setTimeout(() => {
    const before = state.src;
    state.history.push(before);
    state.future = [];
    state.src = code.value;
    setDirty(true);
    store('pm.draft', { src: state.src, filePath: state.filePath });
    render();
  }, 250);
});
code.addEventListener('blur', () => { codeTouched = false; });
code.addEventListener('scroll', () => { $('#gutter').scrollTop = code.scrollTop; });
code.addEventListener('keydown', (e) => {
  if (e.key === 'Tab') { e.preventDefault(); document.execCommand('insertText', false, '  '); }
});

function showDiagnostics() {
  const ol = $('#diag');
  ol.innerHTML = '';
  const errors = state.diagnostics.filter((d) => d.severity === 'Error');
  for (const d of state.diagnostics.slice(0, 40)) {
    const li = document.createElement('li');
    li.className = d.severity === 'Error' ? 'error' : 'warning';
    li.innerHTML = '<span class="where"></span><span class="msg"></span><span class="hint"></span>';
    li.querySelector('.where').textContent = `Baris ${d.line}`;
    li.querySelector('.msg').textContent = d.message;
    li.querySelector('.hint').textContent = d.hint || '';
    li.onclick = () => {
      setCode(true);
      code.focus();
      const start = byteToChar(state.src, d.start);
      const end = byteToChar(state.src, d.end);
      code.setSelectionRange(start, Math.max(end, start + 1));
    };
    ol.appendChild(li);
  }
  if (errors.length && !state.codeOpen) say(`Ada ${errors.length} kesalahan di kode. Buka panel Kode untuk melihatnya.`, true);
}

function byteToChar(s, b) {
  const bytes = new TextEncoder().encode(s);
  return new TextDecoder().decode(bytes.slice(0, b)).length;
}

function setCode(open) {
  state.codeOpen = open;
  $('#code-pane').hidden = !open;
  $('#btn-code').setAttribute('aria-pressed', String(open));
  saveSettings();
  applyZoom();
}
$('#btn-code').addEventListener('click', () => setCode(!state.codeOpen));

function formatCmd() {
  const r = call({ fn: 'format', src: state.src });
  if (r.diagnostics) { say('Kode belum bisa dirapikan karena masih ada kesalahan.', true); return; }
  setSrc(r.src);
  say('Kode sudah dirapikan.');
}
$('#btn-format').addEventListener('click', formatCmd);

// ---------------------------------------------------------------------------
// Berkas dan ekspor

const NEW_SONG = `judul = "Lagu baru"
nada  = C
ketuk = 4

suara S {
}
`;

async function newCmd() {
  if (state.dirty && !window.confirm('Perubahan belum disimpan. Buat lagu baru dan buang perubahan?')) return;
  loadSong(NEW_SONG, null);
  selectAt(0, 0, 'insert');
}

async function openCmd() {
  if (state.dirty && !window.confirm('Perubahan belum disimpan. Buka berkas lain dan buang perubahan?')) return;
  const f = await openFile();
  if (f) loadSong(f.content, f.path);
}

function loadSong(src, path) {
  player.stop();
  state.src = src;
  state.filePath = path;
  state.history = [];
  state.future = [];
  state.sel = null;
  render();
  setDirty(false);
  store('pm.draft', { src, filePath: path });
}

function baseName() {
  const m = (state.doc && state.doc.meta) || {};
  let b = state.filePath ? state.filePath.split(/[\\/]/).pop().replace(/\.[^.]+$/, '') : [m.nomor, m.judul].filter(Boolean).join(' ');
  b = (b || 'lagu').replace(/[\\/:*?"<>|]+/g, '').trim();
  return b || 'lagu';
}

async function save(as = false) {
  const p = await saveText(as ? null : state.filePath, state.src, baseName() + '.nada');
  if (!p) return;
  if (isDesktop) state.filePath = p;
  setDirty(false);
  say(isDesktop ? `Tersimpan di ${p}` : 'Berkas diunduh.');
}

function svgPages(editing) {
  return call({ fn: 'render', src: state.src, opts: opts(editing) });
}

async function toPng(svg, size) {
  const scale = 300 / 72;
  const img = new Image();
  const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }));
  await new Promise((ok, bad) => { img.onload = ok; img.onerror = bad; img.src = url; });
  const c = document.createElement('canvas');
  c.width = Math.round(size[0] * scale);
  c.height = Math.round(size[1] * scale);
  const ctx = c.getContext('2d');
  ctx.fillStyle = 'white';
  ctx.fillRect(0, 0, c.width, c.height);
  ctx.drawImage(img, 0, 0, c.width, c.height);
  URL.revokeObjectURL(url);
  return c.toDataURL('image/png').split(',')[1];
}

function b64utf8(s) {
  const bytes = new TextEncoder().encode(s);
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(bin);
}

async function exportCmd(kind) {
  $('#export-menu').hidden = true;
  $('#btn-export').setAttribute('aria-expanded', 'false');
  const suffix = state.mode === 'tayang' ? ' (tayangan)' : '';
  if (kind === 'print') { window.print(); return; }
  if (kind === 'pdf') {
    say('Menyiapkan PDF...');
    const r = call({ fn: 'pdf', src: state.src, opts: opts(false) });
    const p = await saveBinary(r.pdf, baseName() + suffix + '.pdf', 'pdf');
    if (p) say(isDesktop ? `PDF tersimpan di ${p}` : 'PDF diunduh.');
    return;
  }
  const r = svgPages(false);
  for (let i = 0; i < r.pages.length; i++) {
    const n = r.pages.length > 1 ? `-${i + 1}` : '';
    const name = baseName() + suffix + n + '.' + kind;
    const data = kind === 'png' ? await toPng(r.pages[i], r.size) : b64utf8(r.pages[i]);
    const p = await saveBinary(data, name, kind);
    if (!p) return;
  }
  say(r.pages.length > 1 ? `${r.pages.length} halaman diekspor.` : 'Gambar diekspor.');
}

$('#btn-save').addEventListener('click', () => save(false));
$('#btn-open').addEventListener('click', openCmd);
$('#btn-export').addEventListener('click', () => {
  const m = $('#export-menu');
  m.hidden = !m.hidden;
  $('#btn-export').setAttribute('aria-expanded', String(!m.hidden));
  if (!m.hidden) m.querySelector('button').focus();
});
$$('#export-menu [data-export]').forEach((b) => b.addEventListener('click', () => exportCmd(b.dataset.export)));
document.addEventListener('click', (e) => {
  if (!e.target.closest('.menu-wrap')) { $('#export-menu').hidden = true; $('#btn-export').setAttribute('aria-expanded', 'false'); }
});

// ---------------------------------------------------------------------------
// Tampilan, zoom, putar

$$('.seg [data-mode]').forEach((b) => b.addEventListener('click', () => {
  state.mode = b.dataset.mode;
  $$('.seg [data-mode]').forEach((x) => x.setAttribute('aria-checked', String(x === b)));
  if (state.mode === 'tayang') state.sel = null;
  render();
}));

function zoomValue() {
  if (state.zoom !== 'fit') return state.zoom;
  const avail = stage.clientWidth - 48;
  return Math.max(0.4, Math.min(1.5, Math.floor((avail / (state.size[0] * PT)) * 100) / 100));
}
function applyZoom() { paper.style.zoom = zoomValue(); }
function zoomTo(z) {
  state.zoom = z === 'fit' ? 'fit' : Math.max(0.4, Math.min(2.5, Math.round(z * 10) / 10));
  applyZoom();
  saveSettings();
  say(state.zoom === 'fit' ? 'Halaman dipaskan dengan lebar layar.' : `Perbesaran ${Math.round(state.zoom * 100)}%`);
}
function zoomBy(d) { zoomTo(zoomValue() + d); }
window.addEventListener('resize', applyZoom);
stage.addEventListener('wheel', (e) => {
  if (!e.ctrlKey) return;
  e.preventDefault();
  zoomBy(e.deltaY < 0 ? 0.1 : -0.1);
}, { passive: false });

function togglePlay() {
  if (player.playing) { player.stop(); return; }
  const tl = call({ fn: 'timeline', src: state.src });
  if (!player.play(tl, null)) return say('Belum ada not untuk diputar.');
  $('#btn-play').classList.add('playing');
  $('#btn-play .lbl').textContent = 'Hentikan';
}
player.onStep = (notes) => {
  paper.querySelectorAll('.hit.now').forEach((e) => e.classList.remove('now'));
  for (const n of notes) {
    const el = paper.querySelector(`.hit[data-k="note"][data-v="${n.voice}"][data-i="${n.index}"]`);
    if (el) el.classList.add('now');
  }
};
player.onEnd = () => {
  paper.querySelectorAll('.hit.now').forEach((e) => e.classList.remove('now'));
  $('#btn-play').classList.remove('playing');
  $('#btn-play .lbl').textContent = 'Putar';
};
$('#btn-play').addEventListener('click', togglePlay);
$('#btn-keys').addEventListener('click', () => $('#keys').showModal());

// ---------------------------------------------------------------------------
// Palet: gambar tombol dengan simbol not angka sungguhan

function glyph({ d = '5', beams = 0, up = 0, down = 0, acc = 0, extra = '' }) {
  let s = `<svg viewBox="0 0 30 30" aria-hidden="true"><text x="15" y="22" font-size="17" text-anchor="middle">${d}</text>`;
  for (let i = 0; i < beams; i++) s += `<rect class="s" x="9" y="${6.4 - i * 2.6}" width="12" height="1.2"/>`;
  for (let i = 0; i < up; i++) s += `<circle class="s" cx="15" cy="${beams ? 2 : 5}" r="1.3"/>`;
  for (let i = 0; i < down; i++) s += `<circle class="s" cx="15" cy="26.5" r="1.3"/>`;
  if (acc) s += acc > 0 ? '<line class="k" x1="10.5" y1="23" x2="19.5" y2="10" stroke-width="1.3"/>' : '<line class="k" x1="10.5" y1="10" x2="19.5" y2="23" stroke-width="1.3"/>';
  return s + extra + '</svg>';
}
const T = (x, y, t, size = 17, extra = '') => `<text x="${x}" y="${y}" font-size="${size}" text-anchor="middle" ${extra}>${t}</text>`;
const icons = {
  'dur-960': glyph({}),
  'dur-480': glyph({ beams: 1 }),
  'dur-240': glyph({ beams: 2 }),
  'dur-320': glyph({ beams: 1, extra: T(15, 3.5, '3', 7, 'font-style="italic"') }),
  hold: `<svg viewBox="0 0 30 30" aria-hidden="true">${T(9, 22, '5')}<circle class="s" cx="21" cy="18.5" r="1.4"/></svg>`,
  rest: glyph({ d: '0' }),
  up: glyph({ up: 1 }),
  down: glyph({ down: 1 }),
  sharp: glyph({ d: '4', acc: 1 }),
  flat: glyph({ d: '7', acc: -1 }),
  slur: `<svg viewBox="0 0 30 30" aria-hidden="true">${T(8, 19, '5', 15)}${T(22, 19, '6', 15)}<path class="s" d="M7 23 Q15 29 23 23 Q15 27.4 7 23Z"/></svg>`,
  breath: `<svg viewBox="0 0 30 30" aria-hidden="true">${T(12, 22, '5')}${T(22, 21, '’', 18)}</svg>`,
  fermata: `<svg viewBox="0 0 30 30" aria-hidden="true">${T(15, 25, '5', 15)}<path class="k" d="M9 10 Q15 1 21 10" stroke-width="1.3"/><circle class="s" cx="15" cy="8.5" r="1.2"/></svg>`,
  label: `<svg viewBox="0 0 30 30" aria-hidden="true">${T(15, 11, 'Ref', 9, 'font-style="italic"')}${T(15, 26, '5', 14)}</svg>`,
  'bar-single': '<svg viewBox="0 0 30 30" aria-hidden="true"><rect class="s" x="14.4" y="7" width="1.2" height="17"/></svg>',
  'bar-double': '<svg viewBox="0 0 30 30" aria-hidden="true"><rect class="s" x="12.4" y="7" width="1.2" height="17"/><rect class="s" x="16.4" y="7" width="1.2" height="17"/></svg>',
  'bar-final': '<svg viewBox="0 0 30 30" aria-hidden="true"><rect class="s" x="11.5" y="7" width="1.2" height="17"/><rect class="s" x="15" y="7" width="3.4" height="17"/></svg>',
  'bar-repeatStart': '<svg viewBox="0 0 30 30" aria-hidden="true"><rect class="s" x="9" y="7" width="3.4" height="17"/><rect class="s" x="14" y="7" width="1.2" height="17"/><circle class="s" cx="19" cy="12.5" r="1.3"/><circle class="s" cx="19" cy="18.5" r="1.3"/></svg>',
  'bar-repeatEnd': '<svg viewBox="0 0 30 30" aria-hidden="true"><circle class="s" cx="11" cy="12.5" r="1.3"/><circle class="s" cx="11" cy="18.5" r="1.3"/><rect class="s" x="14.8" y="7" width="1.2" height="17"/><rect class="s" x="17.6" y="7" width="3.4" height="17"/></svg>',
  break: '<svg viewBox="0 0 30 30" aria-hidden="true"><path class="k" d="M22 8v8H9m4-4-4 4 4 4" stroke-width="1.5" fill="none"/></svg>',
  delete: '<svg viewBox="0 0 30 30" aria-hidden="true"><path class="k" d="M10 10l10 10M20 10L10 20" stroke-width="1.6"/></svg>',
};
$$('.tool').forEach((b) => {
  const key = b.dataset.dur ? 'dur-' + b.dataset.dur : b.dataset.bar ? 'bar-' + b.dataset.bar : b.dataset.act;
  b.innerHTML = icons[key] || '';
  b.setAttribute('aria-label', b.title);
  b.addEventListener('mousedown', (e) => e.preventDefault()); // jangan curi fokus dari partitur
  b.addEventListener('click', () => {
    if (b.dataset.dur) setDur(Number(b.dataset.dur));
    else if (b.dataset.bar) bar(b.dataset.bar);
    else act(b.dataset.act);
  });
});

// ---------------------------------------------------------------------------
// Mulai

async function start() {
  await loadEngine();
  const ex = examples();
  const sel = $('#sel-example');
  ex.forEach(([name], i) => {
    const o = document.createElement('option');
    o.value = i;
    o.textContent = name;
    sel.appendChild(o);
  });
  sel.addEventListener('change', () => {
    const i = sel.value;
    sel.value = '';
    if (i === '') return;
    if (state.dirty && !window.confirm('Perubahan belum disimpan. Muat contoh dan buang perubahan?')) return;
    loadSong(ex[Number(i)][1], null);
  });
  setCode(state.codeOpen);
  setDur(BEAT);

  const opened = await pendingFile();
  const draft = load('pm.draft');
  if (opened) loadSong(opened.content, opened.path);
  else if (draft && typeof draft.src === 'string') loadSong(draft.src, draft.filePath || null);
  else loadSong(ex[0][1], null);
  stage.focus({ preventScroll: true });

  window.addEventListener('beforeunload', (e) => {
    if (state.dirty && !isDesktop) { e.preventDefault(); e.returnValue = ''; }
  });
  window.PM = { state, call, render, op, typeDigit, selectAt };
}

start().catch((err) => {
  console.error(err);
  say('Mesin partitur gagal dimuat: ' + err.message, true);
});
