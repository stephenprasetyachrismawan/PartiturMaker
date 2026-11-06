/*
 * Tata letak & penggambaran partitur not angka ke SVG.
 *
 * render(song, opts) -> { pages: [string SVG], page: {w, h} }
 * song adalah hasil PMParser.parse().
 */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.PMRender = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const MM = 96 / 25.4;
  const PAGE_SIZES = {
    A4: { w: 210 * MM, h: 297 * MM, css: 'A4' },
    Letter: { w: 8.5 * 96, h: 11 * 96, css: 'letter' },
    F4: { w: 215 * MM, h: 330 * MM, css: '215mm 330mm' },
    A5: { w: 148 * MM, h: 210 * MM, css: 'A5' },
  };
  const FONT_FAMILY = '"Times New Roman", "Liberation Serif", "Nimbus Roman", Times, serif';
  const EPS = 1e-6;

  let canvasCtx = null;
  function defaultMeasure(text, size, weight, style) {
    if (typeof document !== 'undefined') {
      if (!canvasCtx) canvasCtx = document.createElement('canvas').getContext('2d');
      canvasCtx.font = (style || '') + ' ' + (weight || '') + ' ' + size + 'px ' + FONT_FAMILY;
      return canvasCtx.measureText(text).width;
    }
    return text.length * size * 0.5;
  }

  function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  const r2 = (n) => Math.round(n * 100) / 100;

  function render(song, options) {
    const opts = Object.assign({
      pageSize: 'A4',
      fontSize: 19,
      lyricSize: 15,
      margin: 15 * MM,
      measure: defaultMeasure,
    }, options || {});
    const page = PAGE_SIZES[opts.pageSize] || PAGE_SIZES.A4;
    const fs = opts.fontSize;
    const ls = opts.lyricSize;
    const measure = opts.measure;
    const M = opts.margin;

    const digitW = measure('0', fs);
    const capH = fs * 0.68;
    const dotR = fs * 0.065;
    const dotStep = fs * 0.17;
    const beamGap = fs * 0.15;
    const beamTop = fs * 0.12;
    const beamW = fs * 0.06;
    const thin = Math.max(1, fs * 0.05);

    const voices = song.voices;
    const lyrics = song.lyrics.filter((v) => v.syllables.some(Boolean));
    const nVerse = lyrics.length;

    // ---------- 1. Susun kolom (slot) berdasarkan waktu ----------
    const slotMap = new Map();
    const getSlot = (t, kind) => {
      const key = Math.round(t * 960) * 2 + kind;
      let s = slotMap.get(key);
      if (!s) { s = { key, t, kind, items: [], width: 0 }; slotMap.set(key, s); }
      return s;
    };
    const breakTimes = [];
    voices.forEach((voice, vi) => {
      voice.events.forEach((ev, ei) => {
        ev.vi = vi;
        ev.ei = ei;
        if (ev.type === 'break') {
          if (vi === 0) breakTimes.push(ev.start);
          return;
        }
        const slot = getSlot(ev.start, ev.type === 'bar' ? 0 : 1);
        if (ev.type === 'bar') {
          const prev = slot.items.find((it) => it.vi === vi);
          if (prev) {
            if (prev.ev.barType === ':|' && ev.barType === '|:') prev.ev.barType = ':|:';
            else prev.ev.barType = ev.barType;
            return;
          }
        }
        slot.items.push({ vi, ev });
        ev.slot = slot;
      });
    });
    const slots = Array.from(slotMap.values()).sort((a, b) => a.key - b.key);
    slots.forEach((s, i) => { s.index = i; });

    // ---------- 2. Lebar minimum tiap slot ----------
    const barWidth = { '|': 0.75, '||': 0.9, '|.': 1.0, '|:': 1.35, ':|': 1.35, ':|:': 1.7 };
    for (const s of slots) {
      if (s.kind === 0) {
        let w = 0;
        for (const it of s.items) w = Math.max(w, barWidth[it.ev.barType] || 0.75);
        s.barType = s.items.reduce((acc, it) => (barWidth[it.ev.barType] > (barWidth[acc] || 0) ? it.ev.barType : acc), '|');
        s.width = w * fs;
      } else {
        // Jarak ke kolom berikutnya mengikuti selisih waktunya.
        const next = slots.slice(s.index + 1).find((n) => n.t > s.t + EPS);
        const dt = next ? next.t - s.t : 1;
        let pad = dt >= 1 - EPS ? fs * 0.62 : dt >= 0.5 - EPS ? fs * 0.36 : fs * 0.2;
        // Sedikit lebih lega di batas ketuk (sesudah grup bergaris bawah).
        if (next && dt < 1 - EPS && Math.abs(next.t - Math.round(next.t)) < EPS) pad = Math.max(pad, fs * 0.5);
        s.width = digitW + pad;
      }
    }

    // Lirik butuh ruang: jarak ke suku kata berikutnya >= lebar suku kata.
    const hyphW = measure('-', ls);
    if (voices.length) {
      const melodyEvents = voices[0].events;
      lyrics.forEach((verse, vIdx) => {
        const verseIdx = song.lyrics.indexOf(verse);
        let prev = null;
        let prevW = 0;
        for (const ev of melodyEvents) {
          const syl = ev.lyrics && ev.lyrics[verseIdx];
          if (!syl) continue;
          if (prev) stretchBetween(prev.slot, ev.slot, prevW);
          prev = ev;
          prevW = measure(syl.text, ls) + (syl.hyphen ? hyphW + ls * 0.35 : ls * 0.4);
        }
      });
    }
    function stretchBetween(a, b, need) {
      let have = 0;
      for (let i = a.index; i < b.index; i++) have += slots[i].width;
      const deficit = need - have;
      if (deficit <= 0) return;
      const evSlots = [];
      for (let i = a.index; i < b.index; i++) if (slots[i].kind === 1) evSlots.push(slots[i]);
      const list = evSlots.length ? evSlots : [a];
      for (const s of list) s.width += deficit / list.length;
    }

    // ---------- 3. Pecah menjadi birama, lalu baris ----------
    const units = [];
    let cur = [];
    const breakAt = (t) => breakTimes.some((bt) => Math.abs(bt - t) < EPS);
    const forcedAfter = new Set();
    for (const s of slots) {
      if (s.kind === 0 && s.barType === '|:') {
        if (cur.length) units.push(cur);
        cur = [s];
        if (breakAt(s.t) && units.length) forcedAfter.add(units.length - 1);
        continue;
      }
      if (s.kind === 1 && breakAt(s.t) && cur.length && !cur.some((c) => c.kind === 0 && Math.abs(c.t - s.t) < EPS)) {
        // pindah baris di tengah birama (tanpa garis birama)
        units.push(cur);
        forcedAfter.add(units.length - 1);
        cur = [];
      }
      cur.push(s);
      if (s.kind === 0) {
        units.push(cur);
        if (breakAt(s.t)) forcedAfter.add(units.length - 1);
        cur = [];
      }
    }
    if (cur.length) units.push(cur);

    const prefixW = nVerse ? measure('99.', ls) + ls * 0.5 : 0;
    const x0 = M + prefixW;
    const avail = page.w - M - x0;
    const lines = [];
    let line = [];
    let lineW = 0;
    units.forEach((u, ui) => {
      const w = u.reduce((a, s) => a + s.width, 0);
      if (line.length && lineW + w > avail) {
        lines.push({ slots: line, width: lineW });
        line = [];
        lineW = 0;
      }
      line = line.concat(u);
      lineW += w;
      if (forcedAfter.has(ui)) {
        lines.push({ slots: line, width: lineW, forced: true });
        line = [];
        lineW = 0;
      }
    });
    if (line.length) lines.push({ slots: line, width: lineW });

    // Rata kanan-kiri + posisi x.
    lines.forEach((ln, li) => {
      const isLast = li === lines.length - 1;
      let extra = avail - ln.width;
      // Baris yang terlalu pendek tidak diregangkan agar tidak jarang-jarang.
      if (extra < 0 || ln.width < avail * (isLast ? 0.8 : 0.45)) extra = 0;
      const evSlots = ln.slots.filter((s) => s.kind === 1);
      const evTotal = evSlots.reduce((a, s) => a + s.width, 0) || 1;
      let x = x0;
      for (const s of ln.slots) {
        const w = s.width + (s.kind === 1 ? extra * (s.width / evTotal) : 0);
        s.x = x;
        s.w = w;
        s.cx = s.kind === 0 ? x + w / 2 : x + digitW / 2 + fs * 0.08;
        s.line = li;
        x += w;
      }
      ln.right = x;
    });

    // ---------- 4. Tinggi baris & halaman ----------
    const upper = [];
    const lower = [];
    voices.forEach((v, vi) => ((v.name === 'T' || v.name === 'B') && vi > 0 ? lower : upper).push(vi));
    const rowH = fs * 1.95;
    const baseOff = fs * 1.2;
    const lyrH = ls * 1.4;
    const sysGap = lower.length ? fs * 1.6 : fs * 0.9;
    const systemH = (upper.length + lower.length) * rowH + nVerse * lyrH + (nVerse ? ls * 0.2 : 0);

    // Header
    const meta = song.meta || {};
    const titleSize = fs * 1.3;
    const numSize = fs * 2.1;
    const smallSize = ls * 0.9;
    const header = [];
    let hy = M;
    const hasNum = !!meta.number;
    const titleLines = meta.title ? meta.title.split('\n') : [];
    const subLines = meta.subtitle ? meta.subtitle.split('\n') : [];
    hy += titleSize * 0.95;
    if (hasNum) {
      header.push(text(M, M + numSize * 0.78, meta.number, numSize, { weight: 'bold' }));
    }
    titleLines.forEach((t, i) => {
      header.push(text(page.w / 2, hy, t, titleSize, { weight: 'bold', anchor: 'middle' }));
      hy += titleSize * 1.2;
    });
    if (!titleLines.length) hy += titleSize * 0.2;
    subLines.forEach((t) => {
      header.push(text(page.w / 2, hy - titleSize * 0.25, t, ls, { style: 'italic', anchor: 'middle' }));
      hy += ls * 1.25;
    });
    hy = Math.max(hy, hasNum ? M + numSize * 1.05 : 0);

    const left = [];
    const keyLbl = keyLabel(meta.key);
    const timeLbl = meta.time ? meta.time.trim() : '';
    const leftMain = [keyLbl, timeLbl].filter(Boolean).join(',  ');
    if (leftMain) left.push({ t: leftMain, style: '' });
    if (meta.tempoText) left.push({ t: meta.tempoText, style: 'italic' });
    const right = [];
    const credit = (label, v) => { if (v) v.split('\n').forEach((t, i) => right.push((i ? '' : label) + t)); };
    credit('Syair: ', meta.lyricist);
    credit('Lagu: ', meta.composer);
    credit('Terjemahan: ', meta.translator);
    credit('Aransemen: ', meta.arranger);
    const infoRows = Math.max(left.length, right.length);
    for (let i = 0; i < infoRows; i++) {
      const y = hy + i * smallSize * 1.35 + smallSize;
      if (left[i]) header.push(text(M, y, left[i].t, i === 0 ? ls : smallSize, { style: left[i].style, weight: i === 0 ? 'bold' : '' }));
      if (right[i]) header.push(text(page.w - M, y, right[i], smallSize, { anchor: 'end' }));
    }
    hy += infoRows * smallSize * 1.35 + (infoRows ? fs * 0.9 : fs * 0.4);

    // Footer
    const footerLines = meta.footer ? meta.footer.split('\n') : [];
    const footerH = footerLines.length ? footerLines.length * smallSize * 1.3 + fs * 0.6 : 0;

    // Bagi ke halaman.
    const pages = [{ parts: header.slice(), y0: hy }];
    let y = hy;
    const bottom = page.h - M;
    lines.forEach((ln) => {
      if (y + systemH > bottom && y > (pages.length === 1 ? hy : M) + EPS) {
        pages.push({ parts: [], y0: M });
        y = M;
      }
      ln.page = pages.length - 1;
      ln.y = y;
      y += systemH + sysGap;
    });
    if (footerLines.length) {
      if (y + footerH > bottom + sysGap) {
        pages.push({ parts: [], y0: M });
        y = M;
      }
      const fpage = pages[pages.length - 1];
      footerLines.forEach((t, i) => {
        fpage.parts.push(text(M, y + fs * 0.4 + (i + 1) * smallSize * 1.3, t, smallSize, { style: 'italic' }));
      });
    }
    if (!voices.length) {
      pages[0].parts.push(text(page.w / 2, hy + fs * 2, 'Belum ada not. Tulis baris "S: 1 2 3 4 |" di editor.', ls,
        { anchor: 'middle', style: 'italic', fill: '#888' }));
    }

    // ---------- 5. Gambar setiap baris ----------
    const slurOpen = voices.map(() => null);
    lines.forEach((ln, li) => {
      const out = pages[ln.page].parts;
      const rowBase = {};
      let yy = ln.y;
      upper.forEach((vi) => { rowBase[vi] = yy + baseOff; yy += rowH; });
      const lyrTop = yy;
      yy += nVerse * lyrH + (nVerse ? ls * 0.2 : 0);
      lower.forEach((vi) => { rowBase[vi] = yy + baseOff; yy += rowH; });

      // garis birama
      for (const s of ln.slots) {
        if (s.kind !== 0) continue;
        for (const grp of [upper, lower]) {
          if (!grp.length) continue;
          const top = rowBase[grp[0]] - capH - fs * 0.22;
          const bot = rowBase[grp[grp.length - 1]] + fs * 0.42;
          drawBar(out, s.cx, top, bot, s.barType, grp.map((vi) => rowBase[vi]));
        }
      }

      // not tiap suara
      voices.forEach((voice, vi) => {
        const base = rowBase[vi];
        const evs = [];
        for (const s of ln.slots) {
          if (s.kind !== 1) continue;
          for (const it of s.items) if (it.vi === vi) evs.push(it.ev);
        }
        // balok (garis bawah) per ketuk
        const groups = [];
        let g = null;
        for (const ev of evs) {
          if (ev.div >= 1) {
            const beat = Math.floor(ev.start + EPS);
            const fits = ev.start + ev.dur <= beat + 1 + EPS;
            if (g && g.beat === beat && fits && g.end >= ev.start - EPS) { g.evs.push(ev); g.end = ev.start + ev.dur; }
            else { g = fits ? { beat, evs: [ev], end: ev.start + ev.dur } : null; if (g) groups.push(g); else groups.push({ evs: [ev] }); }
          } else g = null;
        }
        for (const grp of groups) {
          for (let level = 1; level <= 3; level++) {
            let run = [];
            const flush = () => {
              if (!run.length) return;
              const xa = run[0].slot.cx - digitW * 0.55;
              const xb = run[run.length - 1].slot.cx + digitW * 0.55;
              const by = base + beamTop + (level - 1) * beamGap;
              out.push('<rect x="' + r2(xa) + '" y="' + r2(by - beamW / 2) + '" width="' + r2(xb - xa) + '" height="' + r2(beamW) + '"/>');
              run = [];
            };
            for (const ev of grp.evs) { if (ev.div >= level) run.push(ev); else flush(); }
            flush();
          }
        }

        for (const ev of evs) drawEvent(out, ev, ev.slot.cx, base);

        // legato / ikatan
        let open = slurOpen[vi];
        const slurY = (ev) => base - capH - fs * 0.18 - Math.max(0, ev.oct || 0) * dotStep;
        for (const ev of evs) {
          if (ev.type !== 'note') continue;
          if (ev.slurStart) open = ev;
          if (ev.slurEnd && open) {
            const fromPrev = open.slot.line !== li;
            const xa = fromPrev ? ln.slots[0].x - fs * 0.1 : open.slot.cx;
            const ya = fromPrev ? slurY(ev) : slurY(open);
            drawSlur(out, xa, ev.slot.cx, Math.min(ya, slurY(ev)));
            open = null;
          }
        }
        if (open) {
          const xa = open.slot.line === li ? open.slot.cx : ln.slots[0].x;
          drawSlur(out, xa, ln.right + fs * 0.1, slurY(open));
        }
        slurOpen[vi] = open;
      });

      // lirik
      if (nVerse && voices.length) {
        const mel = [];
        for (const s of ln.slots) if (s.kind === 1) for (const it of s.items) if (it.vi === 0) mel.push(it.ev);
        lyrics.forEach((verse, k) => {
          const verseIdx = song.lyrics.indexOf(verse);
          const by = lyrTop + (k + 1) * lyrH - ls * 0.35;
          out.push(text(x0 - ls * 0.4, by, verse.label + '.', ls, { anchor: 'end' }));
          const items = mel.filter((ev) => ev.lyrics && ev.lyrics[verseIdx]);
          items.forEach((ev, j) => {
            const syl = ev.lyrics[verseIdx];
            const lx = ev.slot.cx - digitW / 2;
            out.push(text(lx, by, syl.text, ls, { cls: 'lyr' }));
            if (syl.hyphen) {
              const end = lx + measure(syl.text, ls);
              const next = items[j + 1];
              let hx = end + ls * 0.12;
              if (next) {
                const nx = next.slot.cx - digitW / 2;
                if (nx - end > hyphW * 2.2) hx = (end + nx) / 2 - hyphW / 2;
              }
              out.push(text(hx, by, '-', ls));
            }
          });
        });
      }
    });

    // Nomor halaman
    if (pages.length > 1) {
      pages.forEach((p, i) => {
        p.parts.push(text(page.w / 2, page.h - M / 2, String(i + 1) + ' / ' + pages.length, smallSize, { anchor: 'middle', fill: '#555' }));
      });
    }

    const svgPages = pages.map((p) =>
      '<svg xmlns="http://www.w3.org/2000/svg" class="pm-page" width="' + r2(page.w) + '" height="' + r2(page.h) +
      '" viewBox="0 0 ' + r2(page.w) + ' ' + r2(page.h) + '" font-family=\'' + FONT_FAMILY + '\'>' +
      '<rect class="pm-bg" width="100%" height="100%" fill="#fff"/>' +
      '<g fill="#000" stroke="none">' + p.parts.join('') + '</g></svg>');

    return { pages: svgPages, page };

    // ---------- fungsi bantu gambar ----------
    function text(x, yv, str, size, o) {
      o = o || {};
      let a = '<text x="' + r2(x) + '" y="' + r2(yv) + '" font-size="' + r2(size) + '"';
      if (o.anchor) a += ' text-anchor="' + o.anchor + '"';
      if (o.weight) a += ' font-weight="' + o.weight + '"';
      if (o.style) a += ' font-style="' + o.style + '"';
      if (o.fill) a += ' fill="' + o.fill + '"';
      if (o.cls) a += ' class="' + o.cls + '"';
      return a + '>' + esc(str) + '</text>';
    }

    function dot(out, x, yv, r) {
      out.push('<circle cx="' + r2(x) + '" cy="' + r2(yv) + '" r="' + r2(r || dotR) + '"/>');
    }

    function drawEvent(out, ev, cx, base) {
      const parts = [];
      if (ev.type === 'ext') {
        dot(parts, cx, base - fs * 0.12, fs * 0.075);
      } else {
        const glyph = ev.type === 'rest' ? '0' : String(ev.degree);
        parts.push(text(cx, base, glyph, fs, { anchor: 'middle' }));
        if (ev.type === 'note') {
          for (let k = 0; k < ev.oct; k++) dot(parts, cx, base - capH - fs * 0.14 - k * dotStep);
          for (let k = 0; k < -ev.oct; k++) dot(parts, cx, base + beamTop + ev.div * beamGap + fs * 0.06 + k * dotStep);
          if (ev.acc) {
            const xl = cx - digitW * 0.6;
            const xr = cx + digitW * 0.6;
            const yb = base + fs * 0.05;
            const yt = base - capH - fs * 0.05;
            const [y1, y2] = ev.acc > 0 ? [yb, yt] : [yt, yb];
            parts.push('<line x1="' + r2(xl) + '" y1="' + r2(y1) + '" x2="' + r2(xr) + '" y2="' + r2(y2) +
              '" stroke="#000" stroke-width="' + r2(fs * 0.06) + '"/>');
          }
        }
      }
      if (ev.fermata) {
        const fy = base - capH - fs * 0.35 - Math.max(0, ev.oct || 0) * dotStep - (ev.slurStart || ev.inSlur || ev.slurEnd ? fs * 0.3 : 0);
        const rr = fs * 0.32;
        parts.push('<path d="M' + r2(cx - rr) + ' ' + r2(fy) + ' A' + r2(rr) + ' ' + r2(rr) + ' 0 0 1 ' + r2(cx + rr) + ' ' + r2(fy) +
          '" fill="none" stroke="#000" stroke-width="' + r2(fs * 0.06) + '"/>');
        dot(parts, cx, fy - fs * 0.08, fs * 0.06);
      }
      // area klik
      const hit = '<rect class="hit" x="' + r2(cx - digitW * 0.7) + '" y="' + r2(base - capH - fs * 0.3) + '" width="' + r2(digitW * 1.4) +
        '" height="' + r2(capH + fs * 0.75) + '" fill="transparent"/>';
      out.push('<g class="ev" data-v="' + ev.vi + '" data-i="' + ev.ei + '" data-line="' + ev.line + '" data-col="' + ev.col +
        '" data-len="' + (ev.len || 1) + '">' + parts.join('') + hit + '</g>');
    }

    function drawSlur(out, xa, xb, yv) {
      const len = Math.max(xb - xa, fs * 0.5);
      const h = Math.min(fs * 0.45, fs * 0.18 + len * 0.06);
      const t = fs * 0.06;
      const mx = (xa + xb) / 2;
      out.push('<path d="M' + r2(xa) + ' ' + r2(yv) + ' Q' + r2(mx) + ' ' + r2(yv - h * 2) + ' ' + r2(xb) + ' ' + r2(yv) +
        ' Q' + r2(mx) + ' ' + r2(yv - h * 2 + t * 2) + ' ' + r2(xa) + ' ' + r2(yv) + 'Z"/>');
    }

    function drawBar(out, x, top, bot, type, bases) {
      const line = (xx, w) => out.push('<rect x="' + r2(xx - w / 2) + '" y="' + r2(top) + '" width="' + r2(w) + '" height="' + r2(bot - top) + '"/>');
      const thick = fs * 0.17;
      const dots = (xx) => bases.forEach((b) => {
        dot(out, xx, b - capH * 0.5 - fs * 0.15, fs * 0.07);
        dot(out, xx, b - capH * 0.5 + fs * 0.15, fs * 0.07);
      });
      const u = fs * 0.1;
      switch (type) {
        case '||': line(x - u, thin); line(x + u, thin); break;
        case '|.': line(x - u * 1.3, thin); line(x + u * 0.6, thick); break;
        case '|:': line(x - u * 2, thick); line(x - u * 0.2, thin); dots(x + u * 1.7); break;
        case ':|': dots(x - u * 1.7); line(x + u * 0.2, thin); line(x + u * 2, thick); break;
        case ':|:': dots(x - u * 3.2); line(x - u * 1.6, thin); line(x, thick); line(x + u * 1.6, thin); dots(x + u * 3.2); break;
        default: line(x, thin);
      }
    }
  }

  function keyLabel(keyText) {
    if (!keyText) return '';
    return keyText.indexOf('=') >= 0 ? keyText : 'Do = ' + keyText;
  }

  return { render, PAGE_SIZES, FONT_FAMILY };
});
