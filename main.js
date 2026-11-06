/* Proses utama Electron untuk PartiturMaker. */
'use strict';

const { app, BrowserWindow, Menu, dialog, ipcMain, shell } = require('electron');
const fs = require('fs');
const path = require('path');

const FILE_FILTERS = [
  { name: 'Partitur PartiturMaker', extensions: ['ptm'] },
  { name: 'Teks', extensions: ['txt'] },
  { name: 'Semua berkas', extensions: ['*'] },
];

let win = null;
let dirty = false;
let pendingOpen = null; // berkas yang dibuka lewat klik ganda sebelum jendela siap
let rendererReady = false;

function fileFromArgs(argv) {
  return argv.slice(1).find((a) => /\.(ptm|txt)$/i.test(a) && fs.existsSync(a)) || null;
}

function sendOpen(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    win.webContents.send('open-file', { path: filePath, content });
  } catch (err) {
    dialog.showErrorBox('Gagal membuka berkas', String(err.message || err));
  }
}

function createWindow() {
  win = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 500,
    title: 'PartiturMaker',
    icon: path.join(__dirname, 'build', 'icon.png'),
    backgroundColor: '#eef0f3',
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  win.loadFile(path.join(__dirname, 'src', 'index.html'));
  win.once('ready-to-show', () => win.show());

  // Tautan luar dibuka di browser biasa.
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (/^https?:/.test(url)) shell.openExternal(url);
    return { action: 'deny' };
  });
  win.webContents.on('will-navigate', (e) => e.preventDefault());

  win.on('close', (e) => {
    if (!dirty) return;
    const choice = dialog.showMessageBoxSync(win, {
      type: 'warning',
      buttons: ['Keluar tanpa menyimpan', 'Batal'],
      defaultId: 1,
      cancelId: 1,
      title: 'PartiturMaker',
      message: 'Perubahan belum disimpan.',
      detail: 'Yakin ingin keluar? Perubahan terakhir akan hilang.',
    });
    if (choice !== 0) e.preventDefault();
  });
  win.on('closed', () => { win = null; });
}

function buildMenu() {
  const send = (cmd) => () => win && win.webContents.send('menu', cmd);
  const template = [
    {
      label: '&Berkas',
      submenu: [
        { label: 'Baru', accelerator: 'CmdOrCtrl+N', click: send('new') },
        { label: 'Buka…', accelerator: 'CmdOrCtrl+O', click: send('open') },
        { type: 'separator' },
        { label: 'Simpan', accelerator: 'CmdOrCtrl+S', click: send('save') },
        { label: 'Simpan Sebagai…', accelerator: 'CmdOrCtrl+Shift+S', click: send('saveAs') },
        { type: 'separator' },
        { label: 'Ekspor PDF…', accelerator: 'CmdOrCtrl+E', click: send('pdf') },
        { label: 'Ekspor PNG…', click: send('png') },
        { label: 'Ekspor SVG…', click: send('svg') },
        { label: 'Cetak…', accelerator: 'CmdOrCtrl+P', click: send('print') },
        { type: 'separator' },
        { label: 'Keluar', role: 'quit' },
      ],
    },
    {
      label: '&Edit',
      submenu: [
        { label: 'Urungkan', role: 'undo' },
        { label: 'Ulangi', role: 'redo' },
        { type: 'separator' },
        { label: 'Potong', role: 'cut' },
        { label: 'Salin', role: 'copy' },
        { label: 'Tempel', role: 'paste' },
        { label: 'Pilih Semua', role: 'selectAll' },
      ],
    },
    {
      label: '&Tampilan',
      submenu: [
        { label: 'Perbesar', accelerator: 'CmdOrCtrl+=', click: send('zoomIn') },
        { label: 'Perkecil', accelerator: 'CmdOrCtrl+-', click: send('zoomOut') },
        { label: 'Ukuran Asli', accelerator: 'CmdOrCtrl+0', click: send('zoomReset') },
        { type: 'separator' },
        { label: 'Layar Penuh', role: 'togglefullscreen' },
        { label: 'Alat Pengembang', role: 'toggleDevTools', accelerator: 'CmdOrCtrl+Shift+I' },
      ],
    },
    {
      label: '&Putar',
      submenu: [
        { label: 'Putar', accelerator: 'F5', click: send('play') },
        { label: 'Berhenti', click: send('stop') },
      ],
    },
    {
      label: 'B&antuan',
      submenu: [
        { label: 'Panduan Penulisan Not', accelerator: 'F1', click: send('help') },
        {
          label: 'Tentang PartiturMaker',
          click: () => dialog.showMessageBox(win, {
            type: 'info',
            title: 'Tentang',
            message: 'PartiturMaker ' + app.getVersion(),
            detail: 'Pembuat partitur not angka untuk buku nyanyian jemaat (gaya KJ, PKJ, NKB, dll).',
          }),
        },
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

// ---------- IPC ----------
ipcMain.on('set-dirty', (_e, d) => { dirty = !!d; });

ipcMain.on('renderer-ready', () => {
  rendererReady = true;
  if (pendingOpen) { sendOpen(pendingOpen); pendingOpen = null; }
});

ipcMain.handle('confirm-discard', async () => {
  const { response } = await dialog.showMessageBox(win, {
    type: 'warning',
    buttons: ['Lanjutkan tanpa menyimpan', 'Batal'],
    defaultId: 1,
    cancelId: 1,
    title: 'PartiturMaker',
    message: 'Perubahan belum disimpan.',
    detail: 'Lanjutkan dan buang perubahan?',
  });
  return response === 0;
});

ipcMain.handle('open-file', async () => {
  const { canceled, filePaths } = await dialog.showOpenDialog(win, {
    title: 'Buka partitur',
    filters: FILE_FILTERS,
    properties: ['openFile'],
  });
  if (canceled || !filePaths.length) return null;
  return { path: filePaths[0], content: fs.readFileSync(filePaths[0], 'utf8') };
});

ipcMain.handle('save-file', async (_e, { path: filePath, content, suggestedName }) => {
  let target = filePath;
  if (!target) {
    const res = await dialog.showSaveDialog(win, {
      title: 'Simpan partitur',
      defaultPath: suggestedName || 'lagu.ptm',
      filters: FILE_FILTERS,
    });
    if (res.canceled || !res.filePath) return null;
    target = res.filePath;
  }
  fs.writeFileSync(target, content, 'utf8');
  return target;
});

ipcMain.handle('export-pdf', async (_e, { suggestedName, page }) => {
  const res = await dialog.showSaveDialog(win, {
    title: 'Ekspor PDF',
    defaultPath: suggestedName || 'lagu.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
  if (res.canceled || !res.filePath) return null;
  const data = await win.webContents.printToPDF({
    printBackground: true,
    preferCSSPageSize: true,
    pageSize: page ? { width: page.w / 96, height: page.h / 96 } : 'A4',
    margins: { marginType: 'none' },
  });
  fs.writeFileSync(res.filePath, data);
  return res.filePath;
});

ipcMain.handle('save-files', async (_e, { suggestedName, ext, label, files }) => {
  const res = await dialog.showSaveDialog(win, {
    title: 'Simpan ' + (label || ext.toUpperCase()),
    defaultPath: suggestedName,
    filters: [{ name: label || ext.toUpperCase(), extensions: [ext] }],
  });
  if (res.canceled || !res.filePath) return null;
  const base = res.filePath.replace(new RegExp('\\.' + ext + '$', 'i'), '');
  const written = files.map((f, i) => {
    const p = files.length > 1 ? base + '-' + (i + 1) + '.' + ext : base + '.' + ext;
    if (f.text != null) fs.writeFileSync(p, f.text, 'utf8');
    else fs.writeFileSync(p, Buffer.from(f.base64, 'base64'));
    return p;
  });
  return written;
});

// ---------- Siklus aplikasi ----------
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', (_e, argv) => {
    if (!win) return;
    if (win.isMinimized()) win.restore();
    win.focus();
    const f = fileFromArgs(argv);
    if (f) sendOpen(f);
  });

  app.whenReady().then(() => {
    if (process.platform === 'win32') app.setAppUserModelId('id.partiturmaker.app');
    buildMenu();
    createWindow();
    const f = fileFromArgs(process.argv);
    if (f) {
      if (rendererReady) sendOpen(f);
      else pendingOpen = f;
    }
  });

  app.on('window-all-closed', () => app.quit());
}
