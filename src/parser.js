/*
 * Parser notasi teks PartiturMaker.
 *
 * Format berkas (.ptm) terdiri dari baris "kunci: nilai".
 *   - Kunci header : judul, nomor, nada, birama, syair, lagu, dll.
 *   - Kunci suara  : S, A, T, B (atau M untuk melodi saja).
 *   - Kunci lirik  : 1, 2, 3, ... (nomor bait).
 * Baris yang diawali "#" adalah komentar.
 *
 * Bisa dipakai di browser (window.PMParser) maupun Node (require).
 */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.PMParser = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const HEADER_KEYS = {
    judul: 'title',
    title: 'title',
    nomor: 'number',
    no: 'number',
    subjudul: 'subtitle',
    ayat: 'subtitle',
    nada: 'key',
    do: 'key',
    birama: 'time',
    ketukan: 'time',
    tempo: 'tempoText',
    keterangan: 'tempoText',
    bpm: 'bpm',
    syair: 'lyricist',
    teks: 'lyricist',
    lagu: 'composer',
    musik: 'composer',
    terjemahan: 'translator',
    aransemen: 'arranger',
    catatan: 'footer',
    hakcipta: 'footer',
  };

  const VOICE_KEYS = { s: 'S', m: 'S', a: 'A', t: 'T', b: 'B' };
  const VOICE_ORDER = ['S', 'A', 'T', 'B'];
  const EPS = 1e-6;

  function ParseError(message, line, col) {
    this.message = message;
    this.line = line;
    this.col = col;
  }

  /** Pecah teks menjadi header, baris suara, dan baris lirik. */
  function splitSections(text) {
    const meta = {};
    const voiceSrc = {}; // nama -> [{text, line, col}]
    const lyricSrc = {}; // nomor -> [{text, line}]
    const errors = [];
    const lines = text.replace(/\r\n?/g, '\n').split('\n');

    lines.forEach((raw, i) => {
      const lineNo = i + 1;
      const trimmed = raw.trim();
      if (!trimmed || trimmed.startsWith('#')) return;
      const m = raw.match(/^\s*([^:]+?)\s*:(.*)$/);
      if (!m) {
        errors.push(new ParseError('Baris tidak dikenali (harus "kunci: isi")', lineNo, 1));
        return;
      }
      const key = m[1].trim().toLowerCase();
      const value = m[2];
      const valueCol = raw.indexOf(':') + 2;

      if (HEADER_KEYS[key]) {
        const k = HEADER_KEYS[key];
        meta[k] = meta[k] ? meta[k] + '\n' + value.trim() : value.trim();
      } else if (VOICE_KEYS[key]) {
        const v = VOICE_KEYS[key];
        (voiceSrc[v] = voiceSrc[v] || []).push({ text: value, line: lineNo, col: valueCol });
      } else if (/^\d+$/.test(key)) {
        (lyricSrc[key] = lyricSrc[key] || []).push({ text: value, line: lineNo });
      } else {
        errors.push(new ParseError('Kunci "' + m[1].trim() + '" tidak dikenal', lineNo, 1));
      }
    });
    return { meta, voiceSrc, lyricSrc, errors };
  }

  /** Ubah teks satu suara menjadi daftar token. */
  function tokenizeVoice(parts, errors) {
    const tokens = [];
    for (const part of parts) {
      const s = part.text;
      let i = 0;
      const pos = (j) => ({ line: part.line, col: part.col + j });
      while (i < s.length) {
        const c = s[i];
        if (/\s/.test(c)) { i++; continue; }
        const start = i;
        if (c === '|') {
          let type = '|';
          if (s[i + 1] === '|') { type = '||'; i += 2; }
          else if (s[i + 1] === '.') { type = '|.'; i += 2; }
          else if (s[i + 1] === ':') { type = '|:'; i += 2; }
          else i += 1;
          tokens.push(Object.assign({ kind: 'bar', barType: type }, pos(start)));
          continue;
        }
        if (c === ':' && s[i + 1] === '|') {
          let type = ':|';
          i += 2;
          if (s[i] === ':') { type = ':|:'; i++; }
          tokens.push(Object.assign({ kind: 'bar', barType: type }, pos(start)));
          continue;
        }
        if (c === '\\') {
          tokens.push(Object.assign({ kind: 'break' }, pos(start)));
          i++;
          continue;
        }
        if (c === '(') { tokens.push(Object.assign({ kind: 'slurOpen' }, pos(start))); i++; continue; }
        if (c === ')') { tokens.push(Object.assign({ kind: 'slurClose' }, pos(start))); i++; continue; }
        if (/[0-7.]/.test(c)) {
          const tok = Object.assign({ kind: 'note', digit: c, acc: 0, oct: 0, div: 0, fermata: false }, pos(start));
          i++;
          while (i < s.length && /[#b',\/^]/.test(s[i])) {
            const m = s[i];
            if (m === '#') tok.acc = 1;
            else if (m === 'b') tok.acc = -1;
            else if (m === "'") tok.oct++;
            else if (m === ',') tok.oct--;
            else if (m === '/') tok.div++;
            else if (m === '^') tok.fermata = true;
            i++;
          }
          if (tok.div > 3) errors.push(new ParseError('Terlalu banyak "/" (maksimal 3)', tok.line, tok.col));
          if ((tok.digit === '0' || tok.digit === '.') && (tok.acc || tok.oct)) {
            errors.push(new ParseError('Tanda kres/mol/oktaf tidak berlaku untuk "' + tok.digit + '"', tok.line, tok.col));
          }
          tok.len = i - start;
          tokens.push(tok);
          continue;
        }
        errors.push(new ParseError('Karakter "' + c + '" tidak dikenal di baris not', part.line, part.col + i));
        i++;
      }
    }
    return tokens;
  }

  /** Ubah token menjadi event berwaktu (satuan ketuk). */
  function buildVoice(name, tokens, errors) {
    const events = [];
    let t = 0;
    let slurDepth = 0;
    let pendingSlur = false;
    let lastSounding = null; // event not/istirahat terakhir

    for (const tok of tokens) {
      if (tok.kind === 'bar') {
        events.push({ type: 'bar', barType: tok.barType, start: t, line: tok.line, col: tok.col });
      } else if (tok.kind === 'break') {
        events.push({ type: 'break', start: t, line: tok.line, col: tok.col });
      } else if (tok.kind === 'slurOpen') {
        if (slurDepth > 0) errors.push(new ParseError('Legato "(" bersarang tidak didukung', tok.line, tok.col));
        slurDepth++;
        pendingSlur = true;
      } else if (tok.kind === 'slurClose') {
        if (slurDepth === 0) {
          errors.push(new ParseError('")" tanpa pasangan "("', tok.line, tok.col));
          continue;
        }
        slurDepth--;
        // tutup pada not terakhir
        for (let k = events.length - 1; k >= 0; k--) {
          if (events[k].type === 'note') { events[k].slurEnd = true; break; }
        }
      } else if (tok.kind === 'note') {
        const dur = 1 / Math.pow(2, tok.div);
        const ev = {
          type: tok.digit === '.' ? 'ext' : tok.digit === '0' ? 'rest' : 'note',
          dur,
          start: t,
          div: tok.div,
          fermata: tok.fermata,
          line: tok.line,
          col: tok.col,
          len: tok.len,
        };
        if (ev.type === 'note') {
          ev.degree = Number(tok.digit);
          ev.acc = tok.acc;
          ev.oct = tok.oct;
          if (pendingSlur) { ev.slurStart = true; pendingSlur = false; }
          else if (slurDepth > 0) ev.inSlur = true;
          lastSounding = ev;
        } else if (ev.type === 'rest') {
          lastSounding = ev;
        } else if (ev.type === 'ext') {
          if (!lastSounding) errors.push(new ParseError('Titik "." harus mengikuti not', tok.line, tok.col));
          ev.of = lastSounding;
        }
        events.push(ev);
        t += dur;
      }
    }
    if (slurDepth > 0) {
      const last = tokens[tokens.length - 1];
      errors.push(new ParseError('"(" belum ditutup dengan ")"', last ? last.line : 1, last ? last.col : 1));
    }
    return { name, events, length: t };
  }

  /** Pecah lirik menjadi suku kata. */
  function parseLyrics(parts) {
    const syllables = [];
    const text = parts.map((p) => p.text).join(' ');
    const words = text.split(/\s+/).filter(Boolean);
    for (const w of words) {
      if (w === '_') { syllables.push(null); continue; }
      const pieces = w.split('-');
      pieces.forEach((p, idx) => {
        if (p === '' && idx > 0) return; // "--" atau "-" di akhir
        if (p === '_') { syllables.push(null); return; }
        syllables.push({
          text: p.replace(/~/g, '‿'),
          hyphen: idx < pieces.length - 1 && pieces.slice(idx + 1).some((x) => x !== ''),
        });
      });
    }
    return syllables;
  }

  function parse(text) {
    const { meta, voiceSrc, lyricSrc, errors } = splitSections(text || '');
    const warnings = [];

    const voices = [];
    for (const v of VOICE_ORDER) {
      if (!voiceSrc[v]) continue;
      const tokens = tokenizeVoice(voiceSrc[v], errors);
      voices.push(buildVoice(v, tokens, errors));
    }

    // Lirik ditempelkan ke suara pertama (melodi).
    const lyrics = Object.keys(lyricSrc)
      .sort((a, b) => Number(a) - Number(b))
      .map((label) => ({ label, syllables: parseLyrics(lyricSrc[label]) }));

    if (voices.length) {
      const melody = voices[0];
      const slots = melody.events.filter((e) => e.type === 'note' && !e.inSlur);
      lyrics.forEach((verse, vi) => {
        let idx = 0;
        for (const syl of verse.syllables) {
          if (idx >= slots.length) {
            warnings.push('Bait ' + verse.label + ': suku kata lebih banyak dari jumlah not');
            break;
          }
          if (syl) {
            const ev = slots[idx];
            (ev.lyrics = ev.lyrics || [])[vi] = syl;
          }
          idx++;
        }
      });

      // Cek panjang suara-suara lain.
      for (const v of voices.slice(1)) {
        if (Math.abs(v.length - melody.length) > EPS) {
          warnings.push('Panjang suara ' + v.name + ' (' + fmt(v.length) + ' ketuk) berbeda dengan ' +
            melody.name + ' (' + fmt(melody.length) + ' ketuk)');
        }
      }
    }

    errors.sort((a, b) => a.line - b.line || a.col - b.col);
    return { meta, voices, lyrics, errors, warnings };
  }

  function fmt(n) {
    return String(Math.round(n * 1000) / 1000);
  }

  /** Ambil nada dasar dari teks "Do = G", "Do = Bes", "Fis", dll. Hasil: kelas nada 0-11. */
  function parseKey(keyText) {
    if (!keyText) return 0;
    let s = keyText;
    const eq = s.indexOf('=');
    if (eq >= 0) s = s.slice(eq + 1);
    s = s.trim();
    const m = s.match(/^([A-Ga-g])\s*(is|es|s|#|b|♯|♭)?/);
    if (!m) return 0;
    const base = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 }[m[1].toLowerCase()];
    let acc = 0;
    const suf = (m[2] || '').toLowerCase();
    if (suf === 'is' || suf === '#' || suf === '♯') acc = 1;
    else if (suf === 'es' || suf === 's' || suf === 'b' || suf === '♭') acc = -1;
    return (base + acc + 12) % 12;
  }

  /** Tampilan teks nada: "G" -> "Do = G". */
  function keyLabel(keyText) {
    if (!keyText) return '';
    return keyText.indexOf('=') >= 0 ? keyText : 'Do = ' + keyText;
  }

  return { parse, parseKey, keyLabel, VOICE_ORDER };
});
