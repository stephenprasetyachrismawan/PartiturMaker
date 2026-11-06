'use strict';

const test = require('node:test');
const assert = require('node:assert');
const { parse, parseKey, keyLabel } = require('../src/parser.js');
const { render } = require('../src/render.js');
const examples = require('../src/examples.js');
const { PMPlayerUtil } = require('../src/player.js');

const notes = (voice) => voice.events.filter((e) => e.type !== 'bar' && e.type !== 'break');

test('header dibaca', () => {
  const s = parse('judul: Malam Kudus\nnomor: 92\nnada: Do = Bes\nbirama: 6/8\nsyair: A\nlagu: B\n');
  assert.equal(s.meta.title, 'Malam Kudus');
  assert.equal(s.meta.number, '92');
  assert.equal(s.meta.key, 'Do = Bes');
  assert.equal(s.meta.time, '6/8');
  assert.equal(s.meta.lyricist, 'A');
  assert.equal(s.meta.composer, 'B');
  assert.deepEqual(s.errors, []);
});

test('durasi: ketuk, titik, garis bawah', () => {
  const s = parse('S: 1 . 2/ 3/ 4// 5// 6/ | 7 ./ 1\'/ |');
  assert.deepEqual(s.errors, []);
  const ev = notes(s.voices[0]);
  assert.deepEqual(ev.map((e) => e.dur), [1, 1, 0.5, 0.5, 0.25, 0.25, 0.5, 1, 0.5, 0.5]);
  assert.deepEqual(ev.map((e) => e.type), ['note', 'ext', 'note', 'note', 'note', 'note', 'note', 'note', 'ext', 'note']);
  assert.equal(s.voices[0].length, 6);
});

test('oktaf, kres, mol, fermata', () => {
  const s = parse("S: 1' 5, 1'' 4# 7b 3^");
  const ev = notes(s.voices[0]);
  assert.deepEqual(ev.map((e) => e.oct), [1, -1, 2, 0, 0, 0]);
  assert.deepEqual(ev.map((e) => e.acc), [0, 0, 0, 1, -1, 0]);
  assert.equal(ev[5].fermata, true);
});

test('garis birama dan tanda ulang', () => {
  const s = parse('S: |: 1 2 | 3 4 || 5 6 :| 1 . |.');
  const bars = s.voices[0].events.filter((e) => e.type === 'bar').map((e) => e.barType);
  assert.deepEqual(bars, ['|:', '|', '||', ':|', '|.']);
});

test('lirik dipasang ke not, legato satu suku kata', () => {
  const s = parse('S: 1 2 (3/ 4/) 5 0 6 . 7\n1: Ha-le-lu-ya _ a-min\n');
  assert.deepEqual(s.errors, []);
  // Not penerima: 1 2 3(legato 3-4) 5 6 7. Istirahat & titik dilewati.
  // Suku kata: Ha le lu ya _ a min -> "_" melewati not 6, "a" pada 7, "min" berlebih.
  const sung = s.voices[0].events.filter((e) => e.lyrics).map((e) => [e.degree, e.lyrics[0].text]);
  assert.deepEqual(sung, [[1, 'Ha'], [2, 'le'], [3, 'lu'], [5, 'ya'], [7, 'a']]);
  assert.ok(s.warnings.some((w) => w.includes('lebih banyak')));
});

test('lirik: tanda hubung dan lewati not', () => {
  const s = parse('S: 1 2 3 4\n1: Tu-han _ ba-ik\n');
  const sung = s.voices[0].events.filter((e) => e.lyrics).map((e) => [e.degree, e.lyrics[0].text, e.lyrics[0].hyphen]);
  assert.deepEqual(sung, [[1, 'Tu', true], [2, 'han', false], [4, 'ba', true]]);
});

test('kesalahan dilaporkan dengan nomor baris', () => {
  const s = parse('judul: X\nS: 1 2 x 3\nfoo: bar\nS: (1 2\n');
  const lines = s.errors.map((e) => e.line);
  assert.ok(lines.includes(2), 'karakter x');
  assert.ok(lines.includes(3), 'kunci tak dikenal');
  assert.ok(s.errors.some((e) => e.message.includes('belum ditutup')));
});

test('peringatan bila panjang suara berbeda', () => {
  const s = parse('S: 1 2 3 4 |\nA: 1 2 3 |\n');
  assert.ok(s.warnings.some((w) => w.includes('Panjang suara A')));
});

test('nada dasar', () => {
  assert.equal(parseKey('Do = C'), 0);
  assert.equal(parseKey('Do = G'), 7);
  assert.equal(parseKey('Do = Bes'), 10);
  assert.equal(parseKey('Do = Fis'), 6);
  assert.equal(parseKey('Do = Es'), 3);
  assert.equal(parseKey('As'), 8);
  assert.equal(parseKey('Do = F#'), 6);
  assert.equal(keyLabel('G'), 'Do = G');
  assert.equal(keyLabel('Do = G'), 'Do = G');
});

test('contoh bawaan bebas kesalahan dan bisa digambar', () => {
  for (const ex of examples) {
    const s = parse(ex.text);
    assert.deepEqual(s.errors, [], ex.id);
    assert.deepEqual(s.warnings, [], ex.id);
    const out = render(s, { pageSize: 'A4' });
    assert.ok(out.pages.length >= 1);
    assert.match(out.pages[0], /^<svg/);
    assert.ok(out.pages[0].includes(s.meta.title));
  }
});

test('lagu panjang dibagi ke beberapa halaman', () => {
  const bar = 'S: ' + Array(40).fill("1 2 3 4 | 5 6 7 1' |").join(' ') + '\n';
  const s = parse('judul: Panjang\n' + bar + '1: ' + Array(320).fill('la').join(' ') + '\n');
  const out = render(s, { pageSize: 'A5' });
  assert.ok(out.pages.length > 1);
  assert.ok(out.pages[1].includes(' / '), 'nomor halaman');
});

test('pemutar: titik memperpanjang, ikatan nada sama digabung', () => {
  const s = parse('S: 1 . (5 5) 0 3');
  const n = PMPlayerUtil.voiceToNotes(s.voices[0], 60);
  assert.deepEqual(n.map((x) => [x.midi, x.start, x.dur]), [[60, 0, 2], [67, 2, 2], [64, 5, 1]]);
});
