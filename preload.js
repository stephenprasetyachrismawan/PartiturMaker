/* Jembatan aman antara jendela (renderer) dan proses utama. */
'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('pmNative', {
  openFile: () => ipcRenderer.invoke('open-file'),
  saveFile: (args) => ipcRenderer.invoke('save-file', args),
  exportPdf: (args) => ipcRenderer.invoke('export-pdf', args),
  saveFiles: (args) => ipcRenderer.invoke('save-files', args),
  confirmDiscard: () => ipcRenderer.invoke('confirm-discard'),
  setDirty: (d) => ipcRenderer.send('set-dirty', d),
  ready: () => ipcRenderer.send('renderer-ready'),
  onMenu: (cb) => ipcRenderer.on('menu', (_e, cmd) => cb(cmd)),
  onOpenFile: (cb) => ipcRenderer.on('open-file', (_e, res) => cb(res)),
});
