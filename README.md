# PartiturMaker

Aplikasi Windows untuk membuat partitur **not angka** bergaya buku nyanyian jemaat
(Kidung Jemaat, Pelengkap Kidung Jemaat, Nyanyikanlah Kidung Baru, dan sejenisnya).

![Tampilan PartiturMaker](docs/screenshot.png)

## Fitur

- Tulis lagu sebagai teks sederhana, pratinjau partitur langsung muncul di sebelahnya.
- Header gaya buku nyanyian: nomor lagu, judul, ayat, `Do = …`, birama, tempo, syair & lagu.
- 1 sampai 4 suara (Sopran, Alto, Tenor, Bas). Lirik diletakkan di antara alto dan tenor.
- Beberapa bait lirik, dengan tanda hubung suku kata otomatis.
- Simbol not angka lengkap: titik oktaf atas/bawah, garis bawah ½ dan ¼ ketuk, titik
  perpanjangan, kres (/) dan mol (\), legato/ikatan, fermata, istirahat, birama ganda,
  garis akhir, tanda ulang.
- Pindah baris dan halaman otomatis, baris dirata kanan-kiri.
- Ekspor **PDF**, **PNG** (300 dpi), **SVG**, dan cetak langsung. Kertas A4, F4/Folio, Letter, A5.
- Putar lagu (semua suara atau satu suara saja) dengan sorotan not yang sedang berbunyi.
- Klik not di pratinjau untuk melompat ke teksnya. Kesalahan ditunjukkan per baris.
- Berkas `.ptm` adalah teks biasa (bisa dibuka di Notepad) dan terdaftar ke aplikasi saat instal.

## Unduh & pasang (Windows)

Installer dibangun otomatis oleh GitHub Actions di setiap push:

1. Buka tab **Actions** → workflow **Build Windows** → pilih proses terbaru.
2. Unduh artefak **PartiturMaker-Windows**, lalu ekstrak.
3. Jalankan `PartiturMaker-Setup-x.y.z.exe` (installer) atau
   `PartiturMaker-Portable-x.y.z.exe` (tanpa instal).

Untuk membuat rilis resmi, buat tag `v1.0.0` lalu push; berkas `.exe` akan dilampirkan ke halaman Releases.

> Aplikasi belum ditandatangani (code signing), jadi Windows SmartScreen bisa menampilkan
> peringatan. Klik **More info → Run anyway**.

## Cara menulis lagu

```
nomor: 1
judul: Muliakan Nama-Nya
subjudul: Mazmur 100
nada: Do = G
birama: 4/4
tempo: Khidmat
syair: Nama Penulis
lagu: Nama Komponis

S: 1 1 3 5 | 6 5 3 . | 6, 1 2 3 | 2 1 2 . |
A: 5, 6, 1 3 | 4 3 1 . | 4, 5, 7, 1 | 7, 1 7, . |

1: Mu-li-a-kan na-ma-Nya, ha-i u-mat se-mu-a!
2: Na-nyi-kan la-gu ba-ru de-ngan se-ge-nap ha-ti;
```

| Tulis | Arti |
|---|---|
| `1`–`7`, `0` | Not 1 ketuk, `0` = istirahat |
| `.` | Perpanjang 1 ketuk (`5 . . .` = 4 ketuk) |
| `1'` `5,` | Oktaf tinggi / rendah (boleh diulang: `1''`, `5,,`) |
| `3/` `3//` | ½ ketuk / ¼ ketuk (garis bawah) |
| `1 ./ 2/` | Not bertitik (1½ + ½ ketuk) |
| `4#` `7b` | Kres / mol |
| `(4/ 3/)` | Legato: satu suku kata untuk beberapa not |
| `1^` | Fermata |
| `\|` `\|\|` `\|.` `\|:` `:\|` | Garis birama, ganda, akhir, ulang |
| `\` | Paksa pindah baris |

Lirik: `1:` adalah nomor bait. Pisahkan suku kata dengan `-`, pakai `_` untuk melewati
satu not, dan `~` untuk menyambung dua kata pada satu not. Panduan lengkap ada di tombol
**? Panduan** (F1) di dalam aplikasi.

Tenor dan bas ditulis seperti kebiasaan buku nyanyian (satu oktaf lebih tinggi dari bunyinya);
saat diputar, keduanya dibunyikan satu oktaf lebih rendah.

## Pengembangan

Butuh Node.js 20+.

```bash
npm install
```

```bash
npm start
```

```bash
npm test
```

```bash
npm run dist
```

`npm run dist` menghasilkan installer di folder `dist/`. Installer NSIS paling mudah dibangun
di Windows (di Linux butuh `wine`; versi portable bisa dibangun tanpa `wine`).

Struktur:

- `main.js`, `preload.js` — proses utama Electron (menu, dialog berkas, ekspor PDF).
- `src/parser.js` — membaca teks lagu.
- `src/render.js` — tata letak & gambar SVG not angka.
- `src/player.js` — pemutar WebAudio.
- `src/app.js` — antarmuka.

Folder `src/` juga bisa dibuka langsung di browser (`src/index.html`) untuk mencoba tanpa Electron.

## Catatan hak cipta

Contoh lagu bawaan dibuat khusus untuk aplikasi ini. Syair dan lagu dalam KJ, PKJ, NKB, dll.
umumnya dilindungi hak cipta (mis. milik YAMUGER); pastikan Anda punya izin sebelum
memperbanyak atau membagikannya.
