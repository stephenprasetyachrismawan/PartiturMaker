/* Logika antarmuka PartiturMaker. */
(function () {
  'use strict';

  const $ = (id) => document.getElementById(id);
  const native = window.pmNative || null; // tersedia bila berjalan di Electron
  const editor = $('editor');
  const gutter = $('gutter');
  const pagesEl = $('pages');
  const statusEl = $('status');
  const infoEl = $('status-info');
  const player = new window.PMPlayer();

  const SETTINGS_KEY = 'pm.settings';
  const DRAFT_KEY = 'pm.draft';

  const state = {
    filePath: null,
    dirty: false,
    song: null,
    lastRender: null,
    settings: Object.assign({ pageSize: 'A4', fontSize: 19, lyricSize: 15, zoom: 'fit', split: 40 }, loadJSON(SETTINGS_KEY)),
  };

  function loadJSON(key) {
    try { return JSON.parse(localStorage.getItem(key)) || {}; } catch (e) { return {}; }
  }
  function saveJSON(key, v) {
    try { localStorage.setItem(key, JSON.stringify(v)); } catch (e) { /* abaikan */ }
  }

  // ---------- Render ----------
  let renderTimer = null;
  function scheduleRender() {
    clearTimeout(renderTimer);
    renderTimer = setTimeout(renderNow, 180);
  }

  function renderNow() {
    const text = editor.value;
    const song = window.PMParser.parse(text);
    state.song = song;
    let result;
    try {
      result = window.PMRender.render(song, {
        pageSize: state.settings.pageSize,
        fontSize: state.settings.fontSize,
        lyricSize: state.settings.lyricSize,
      });
    } catch (err) {
      console.error(err);
      setStatus('Gagal menggambar partitur: ' + err.message, 'error');
      return;
    }
    state.lastRender = result;
    pagesEl.innerHTML = result.pages.join('');
    updatePageStyle(result.page);
    updateGutter(song.errors);

    if (song.errors.length) {
      const e = song.errors[0];
      setStatus('Baris ' + e.line + ', kolom ' + e.col + ': ' + e.message +
        (song.errors.length > 1 ? '  (+' + (song.errors.length - 1) + ' kesalahan lain)' : ''), 'error', e);
    } else if (song.warnings.length) {
      setStatus('Peringatan: ' + song.warnings[0] + (song.warnings.length > 1 ? '  (+' + (song.warnings.length - 1) + ')' : ''), 'warn');
    } else {
      setStatus('Siap', 'ok');
    }
    const beats = song.voices.length ? song.voices[0].length : 0;
    infoEl.textContent = song.voices.map((v) => v.name).join(' ') + (song.voices.length ? ' · ' : '') +
      Math.round(beats * 100) / 100 + ' ketuk · ' + result.pages.length + ' halaman';
  }

  function updatePageStyle(page) {
    const wIn = page.w / 96;
    const hIn = page.h / 96;
    $('page-style').textContent = '@page { size: ' + wIn.toFixed(3) + 'in ' + hIn.toFixed(3) + 'in; margin: 0; }' +
      '.pages { zoom: ' + effectiveZoom() + '; }';
    $('zoom-label').textContent = (state.settings.zoom === 'fit' ? 'Pas ' : '') + Math.round(effectiveZoom() * 100) + '%';
  }

  function effectiveZoom() {
    if (state.settings.zoom !== 'fit') return state.settings.zoom;
    const page = state.lastRender ? state.lastRender.page : { w: 794 };
    const avail = document.querySelector('.preview-pane').clientWidth - 56;
    return Math.max(0.3, Math.min(2, Math.floor((avail / page.w) * 100) / 100));
  }

  function updateGutter(errors) {
    const n = editor.value.split('\n').length;
    const errLines = new Set(errors.map((e) => e.line));
    let html = '';
    for (let i = 1; i <= n; i++) html += errLines.has(i) ? '<span class="err">' + i + '</span>\n' : i + '\n';
    gutter.innerHTML = html;
    gutter.scrollTop = editor.scrollTop;
  }

  let statusTarget = null;
  function setStatus(msg, kind, target) {
    statusEl.textContent = msg;
    statusEl.className = 'status' + (kind ? ' ' + kind : '');
    statusTarget = target || null;
  }
  statusEl.addEventListener('click', () => { if (statusTarget) selectSource(statusTarget.line, statusTarget.col, 1); });

  function selectSource(line, col, len) {
    const lines = editor.value.split('\n');
    let pos = 0;
    for (let i = 0; i < line - 1 && i < lines.length; i++) pos += lines[i].length + 1;
    pos += Math.max(0, col - 1);
    editor.focus();
    editor.setSelectionRange(pos, pos + (len || 1));
    const lineH = parseFloat(getComputedStyle(editor).lineHeight) || 22;
    const target = (line - 1) * lineH - editor.clientHeight / 3;
    if (target > editor.scrollTop + editor.clientHeight - lineH * 3 || target < editor.scrollTop - lineH) editor.scrollTop = Math.max(0, target);
  }

  // Klik not -> lompat ke teks
  pagesEl.addEventListener('click', (e) => {
    const g = e.target.closest('.ev');
    if (!g) return;
    selectSource(Number(g.dataset.line), Number(g.dataset.col), Number(g.dataset.len));
  });

  // ---------- Editor ----------
  editor.addEventListener('input', () => {
    setDirty(true);
    scheduleRender();
    saveJSON(DRAFT_KEY, { text: editor.value, filePath: state.filePath });
  });
  editor.addEventListener('scroll', () => { gutter.scrollTop = editor.scrollTop; });
  editor.addEventListener('keydown', (e) => {
    if (e.key === 'Tab' && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      document.execCommand('insertText', false, '  ');
    }
  });

  function setDirty(d) {
    state.dirty = d;
    const name = state.filePath ? state.filePath.split(/[\\/]/).pop() : 'Tanpa judul';
    $('file-name').textContent = name + (d ? ' •' : '');
    document.title = (d ? '• ' : '') + name + ' — PartiturMaker';
    if (native) native.setDirty(d);
  }

  function setText(text, filePath) {
    player.stop();
    editor.value = text;
    state.filePath = filePath || null;
    setDirty(false);
    editor.scrollTop = 0;
    renderNow();
    saveJSON(DRAFT_KEY, { text, filePath: state.filePath });
  }

  async function confirmDiscard() {
    if (!state.dirty) return true;
    if (native) return native.confirmDiscard();
    return window.confirm('Perubahan belum disimpan. Lanjutkan tanpa menyimpan?');
  }

  // ---------- Berkas ----------
  const NEW_TEMPLATE = 'nomor: \njudul: Judul Lagu\nnada: Do = C\nbirama: 4/4\nsyair: \nlagu: \n\nS: 1 2 3 4 | 5 . . . |\n\n1: Ki-ta ber-nya-nyi\n';

  async function cmdNew() {
    if (!(await confirmDiscard())) return;
    setText(NEW_TEMPLATE, null);
    editor.focus();
  }

  async function cmdOpen() {
    if (!(await confirmDiscard())) return;
    if (native) {
      const res = await native.openFile();
      if (res) setText(res.content, res.path);
      return;
    }
    const inp = document.createElement('input');
    inp.type = 'file';
    inp.accept = '.ptm,.txt';
    inp.onchange = () => {
      const f = inp.files[0];
      if (!f) return;
      f.text().then((t) => setText(t, f.name));
    };
    inp.click();
  }

  function suggestedName(ext) {
    const meta = (state.song && state.song.meta) || {};
    let base = state.filePath ? state.filePath.split(/[\\/]/).pop().replace(/\.[^.]+$/, '') : '';
    if (!base) base = [meta.number, meta.title].filter(Boolean).join(' ') || 'lagu';
    base = base.replace(/[\\/:*?"<>|]+/g, '').trim() || 'lagu';
    return base + '.' + ext;
  }

  async function cmdSave(saveAs) {
    if (native) {
      const p = await native.saveFile({ path: saveAs ? null : state.filePath, content: editor.value, suggestedName: suggestedName('ptm') });
      if (p) {
        state.filePath = p;
        setDirty(false);
        setStatus('Tersimpan: ' + p, 'ok');
      }
      return;
    }
    downloadBlob(new Blob([editor.value], { type: 'text/plain' }), suggestedName('ptm'));
    setDirty(false);
  }

  function downloadBlob(blob, name) {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = name;
    document.body.appendChild(a);
    a.click();
    setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
  }

  // ---------- Ekspor ----------
  async function cmdPdf() {
    renderNow();
    if (native) {
      const p = await native.exportPdf({ suggestedName: suggestedName('pdf'), page: state.lastRender.page });
      if (p) setStatus('PDF tersimpan: ' + p, 'ok');
      return;
    }
    window.print();
  }

  function cmdPrint() {
    renderNow();
    window.print();
  }

  function svgToPng(svg, page, scale) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }));
      img.onload = () => {
        const c = document.createElement('canvas');
        c.width = Math.round(page.w * scale);
        c.height = Math.round(page.h * scale);
        const ctx = c.getContext('2d');
        ctx.fillStyle = '#fff';
        ctx.fillRect(0, 0, c.width, c.height);
        ctx.drawImage(img, 0, 0, c.width, c.height);
        URL.revokeObjectURL(url);
        resolve(c.toDataURL('image/png'));
      };
      img.onerror = (e) => { URL.revokeObjectURL(url); reject(e); };
      img.src = url;
    });
  }

  function cleanSvg(svg) {
    return '<?xml version="1.0" encoding="UTF-8"?>\n' +
      svg.replace(/<rect class="hit"[^>]*\/>/g, '').replace(/ class="ev( playing)?"/g, '');
  }

  async function cmdPng() {
    renderNow();
    setStatus('Membuat gambar…');
    const scale = 300 / 96; // 300 dpi
    const pages = state.lastRender.pages.map(cleanSvg);
    const images = [];
    for (const svg of pages) images.push(await svgToPng(svg, state.lastRender.page, scale));
    await saveOutputs('png', images.map((d) => ({ base64: d.split(',')[1] })), 'Gambar PNG');
  }

  async function cmdSvg() {
    renderNow();
    const pages = state.lastRender.pages.map(cleanSvg);
    await saveOutputs('svg', pages.map((s) => ({ text: s })), 'Gambar SVG');
  }

  async function saveOutputs(ext, files, label) {
    const name = suggestedName(ext);
    if (native) {
      const paths = await native.saveFiles({ suggestedName: name, ext, label, files });
      if (paths && paths.length) setStatus(label + ' tersimpan: ' + paths.join(', '), 'ok');
      else setStatus('Siap');
      return;
    }
    files.forEach((f, i) => {
      const n = files.length > 1 ? name.replace('.' + ext, '-' + (i + 1) + '.' + ext) : name;
      const blob = f.text != null
        ? new Blob([f.text], { type: 'image/svg+xml' })
        : new Blob([Uint8Array.from(atob(f.base64), (c) => c.charCodeAt(0))], { type: 'image/png' });
      downloadBlob(blob, n);
    });
    setStatus(label + ' diunduh', 'ok');
  }

  // ---------- Putar ----------
  player.onStep = (events) => {
    pagesEl.querySelectorAll('.ev.playing').forEach((g) => g.classList.remove('playing'));
    for (const ev of events) {
      const g = pagesEl.querySelector('.ev[data-v="' + ev.vi + '"][data-i="' + ev.ei + '"]');
      if (g) g.classList.add('playing');
    }
  };
  player.onEnd = () => {
    pagesEl.querySelectorAll('.ev.playing').forEach((g) => g.classList.remove('playing'));
    $('btn-play').disabled = false;
    $('btn-stop').disabled = true;
  };

  function cmdPlay() {
    renderNow();
    if (!state.song || !state.song.voices.length) return;
    const sel = $('sel-voices').value;
    player.play(state.song, { voices: sel ? [sel] : null });
    if (player.playing) {
      $('btn-play').disabled = true;
      $('btn-stop').disabled = false;
    }
  }
  function cmdStop() { player.stop(); }

  // ---------- Pengaturan ----------
  function applySettings() {
    $('sel-page').value = state.settings.pageSize;
    $('rng-font').value = state.settings.fontSize;
    $('rng-lyric').value = state.settings.lyricSize;
    document.querySelector('.editor-pane').style.width = state.settings.split + '%';
  }
  function changeSetting(k, v) {
    state.settings[k] = v;
    saveJSON(SETTINGS_KEY, state.settings);
    applySettings();
    renderNow();
  }
  function zoom(delta) {
    const z = Math.min(3, Math.max(0.3, Math.round((effectiveZoom() + delta) * 10) / 10));
    changeSetting('zoom', z);
  }

  $('sel-page').addEventListener('change', (e) => changeSetting('pageSize', e.target.value));
  $('rng-font').addEventListener('input', (e) => changeSetting('fontSize', Number(e.target.value)));
  $('rng-lyric').addEventListener('input', (e) => changeSetting('lyricSize', Number(e.target.value)));
  $('btn-zoom-in').addEventListener('click', () => zoom(0.1));
  $('btn-zoom-out').addEventListener('click', () => zoom(-0.1));

  // Penggeser lebar panel
  const splitter = $('splitter');
  splitter.addEventListener('pointerdown', (e) => {
    splitter.setPointerCapture(e.pointerId);
    splitter.classList.add('drag');
    const move = (ev) => {
      const pct = Math.min(75, Math.max(20, (ev.clientX / window.innerWidth) * 100));
      state.settings.split = Math.round(pct);
      document.querySelector('.editor-pane').style.width = pct + '%';
    };
    const up = () => {
      splitter.classList.remove('drag');
      splitter.removeEventListener('pointermove', move);
      splitter.removeEventListener('pointerup', up);
      saveJSON(SETTINGS_KEY, state.settings);
      if (state.lastRender) updatePageStyle(state.lastRender.page);
    };
    splitter.addEventListener('pointermove', move);
    splitter.addEventListener('pointerup', up);
  });

  // ---------- Contoh ----------
  const selEx = $('sel-example');
  window.PMExamples.forEach((ex) => {
    const o = document.createElement('option');
    o.value = ex.id;
    o.textContent = ex.name;
    selEx.appendChild(o);
  });
  selEx.addEventListener('change', async () => {
    const ex = window.PMExamples.find((x) => x.id === selEx.value);
    selEx.value = '';
    if (ex && (await confirmDiscard())) setText(ex.text, null);
  });

  // ---------- Panduan ----------
  const help = $('help');
  const toggleHelp = (show) => { help.hidden = show === undefined ? !help.hidden : !show; };
  $('btn-help').addEventListener('click', () => toggleHelp());
  $('btn-help-close').addEventListener('click', () => toggleHelp(false));
  help.addEventListener('click', (e) => { if (e.target === help) toggleHelp(false); });

  // ---------- Tombol & pintasan ----------
  const commands = {
    new: cmdNew,
    open: cmdOpen,
    save: () => cmdSave(false),
    saveAs: () => cmdSave(true),
    pdf: cmdPdf,
    png: cmdPng,
    svg: cmdSvg,
    print: cmdPrint,
    play: cmdPlay,
    stop: cmdStop,
    help: () => toggleHelp(),
    zoomIn: () => zoom(0.1),
    zoomOut: () => zoom(-0.1),
    zoomReset: () => changeSetting('zoom', 'fit'),
  };
  const bind = (id, cmd) => $(id).addEventListener('click', () => commands[cmd]());
  bind('btn-new', 'new');
  bind('btn-open', 'open');
  bind('btn-save', 'save');
  bind('btn-play', 'play');
  bind('btn-stop', 'stop');
  bind('btn-pdf', 'pdf');
  bind('btn-png', 'png');
  bind('btn-svg', 'svg');
  bind('btn-print', 'print');

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (!help.hidden) toggleHelp(false);
      else cmdStop();
      return;
    }
    if (e.key === 'F5') { e.preventDefault(); cmdPlay(); return; }
    if (e.key === 'F1') { e.preventDefault(); toggleHelp(); return; }
    if (native) return; // di Electron pintasan Ctrl ditangani menu
    if (!e.ctrlKey && !e.metaKey) return;
    const k = e.key.toLowerCase();
    const map = { n: 'new', o: 'open', s: e.shiftKey ? 'saveAs' : 'save', p: 'print' };
    if (map[k]) { e.preventDefault(); commands[map[k]](); }
  });

  if (native) {
    native.onMenu((cmd) => { if (commands[cmd]) commands[cmd](); });
    native.onOpenFile((res) => {
      confirmDiscard().then((ok) => { if (ok) setText(res.content, res.path); });
    });
  } else {
    window.addEventListener('beforeunload', (e) => {
      if (state.dirty) { e.preventDefault(); e.returnValue = ''; }
    });
  }

  window.addEventListener('resize', () => { if (state.lastRender) updatePageStyle(state.lastRender.page); });
  $('zoom-label').addEventListener('click', () => changeSetting('zoom', 'fit'));

  // ---------- Mulai ----------
  applySettings();
  const draft = loadJSON(DRAFT_KEY);
  if (draft && typeof draft.text === 'string' && draft.text.trim()) {
    editor.value = draft.text;
    state.filePath = draft.filePath || null;
    setDirty(false);
    renderNow();
  } else {
    setText(window.PMExamples[0].text, null);
  }
  if (native) native.ready();

  // Untuk pengujian otomatis
  window.PMApp = { state, renderNow, setText, commands };
})();
