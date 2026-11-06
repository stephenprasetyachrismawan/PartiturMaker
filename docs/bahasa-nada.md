# Bahasa Nada

Nada adalah bahasa teks kecil untuk menulis partitur not angka. Setiap suntingan di
partitur (klik, ketik angka, isi lirik) diubah PartiturMaker menjadi kode Nada, dan
setiap kode Nada bisa digambar menjadi partitur. Berkasnya berakhiran `.nada` dan bisa
dibuka di Notepad.

```text
// Komentar diawali dua garis miring.
nomor = "KJ 1"
judul = "Haleluya! Pujilah"
nada  = F
ketuk = 4

suara S {
  3 [5 6] 3 [1 2] | 3 [5 6] 3 . ; |
}

bait 1 {
  Ha-le-lu-ya! Pu-ji-lah Al-lah
}
```

## Keterangan lagu

Ditulis `kunci = nilai`. Teks yang berisi spasi diapit tanda petik.

| Kunci | Isi | Tampil di partitur |
|---|---|---|
| `nomor` | `"KJ 1"` | kiri atas, tebal |
| `judul` | `"Haleluya! Pujilah"` | tengah atas |
| `ayat` | `"Mazmur 100"` | di bawah judul, miring |
| `nada` | `F`, `Bes`, `Fis`, `Es`, `As` | `do = f` di kanan |
| `ketuk` | `4` atau `6/8` | `4 ketuk` di kanan |
| `tempo` | `"Khidmat"` | kiri, miring |
| `bpm` | `88` | kecepatan saat diputar |
| `syair`, `lagu`, `terjemahan`, `aransemen` | nama | di bawah lagu |
| `catatan` | `"Hak cipta ..."` | paling bawah |

## Not

Satu token = satu ketuk.

| Tulis | Arti | Gambar |
|---|---|---|
| `1` … `7` | not | angka |
| `0` | tanda diam | `0` |
| `.` | tahan satu ketuk lagi | titik |
| `1'` `1''` | oktaf atas | titik di atas angka |
| `5,` `5,,` | oktaf bawah | titik di bawah angka |
| `4#` | kres | garis miring `/` menembus angka |
| `7b` | mol | garis miring `\` menembus angka |
| `5^` | fermata | lengkung bertitik di atas |

## Membagi ketuk dengan `[ ]`

Kurung siku membagi satu ketuk sama rata. Setiap tingkat kurung menjadi satu garis
harga **di atas** angka, seperti di Kidung Jemaat.

| Tulis | Arti |
|---|---|
| `[5 6]` | dua not ½ ketuk (satu garis) |
| `[1 2 3 4]` | empat not ¼ ketuk (dua garis) |
| `[[1 2] 3]` | ¼ + ¼ + ½ |
| `[. 6]` | titik ½ + not ½ (irama bertitik: `5 [. 6]`) |
| `[5 6 7]` | triol, angka 3 di atas garis |

## Tanda lain

| Tulis | Arti |
|---|---|
| `(5 6)` | legato atau ikatan, lengkung di bawah angka. Hanya satu suku kata. |
| `;` | tanda napas `’` sesudah not sebelumnya |
| `@Refrein` atau `@"Ulang 2x"` | label kecil di atas not berikutnya |
| `\|` `\|\|` `\|.` | garis birama, birama ganda, garis akhir |
| `\|:` `:\|` `:\|:` | tanda ulang |
| `\` | paksa pindah baris di partitur |

## Suara

`suara S`, `suara A`, `suara T`, `suara B` (boleh juga `sopran`, `alto`, `tenor`, `bas`).
Lirik selalu mengikuti suara pertama dan dicetak di antara alto dan tenor. Tenor dan bas
ditulis seperti kebiasaan buku nyanyian; saat diputar keduanya dibunyikan satu oktaf lebih
rendah.

## Lirik

```text
bait 1 {
  Ha-le-lu-ya! Pu-ji-lah Al-lah yang A-gung
}
```

- Satu suku kata untuk satu not. Not di dalam legato dihitung satu.
- `-` memisahkan suku kata dan dicetak sebagai tanda hubung.
- `_` melewati satu not.
- `~` menyambung dua kata pada satu not (`ku~a`).

Bait yang dicetak sebagai teks biasa di bawah lagu:

```text
teks 3 {
  Bersama s'gala ciptaan, kita bersorak sorai;
  nama-Nya terpujilah, kini dan selamanya.
}
```

## Formatter

Tombol **Rapikan** (atau `cargo run --example nadafmt -- lagu.nada`) menulis ulang kode
menjadi bentuk baku:

- keterangan lagu berurutan baku, tanda `=` sejajar;
- empat birama per baris, kecuali Anda memakai `\`;
- not yang melewati batas ketuk dipecah menjadi not dan titik, sehingga garis harganya benar;
- baris lirik dipotong mengikuti baris not.

Menjalankan formatter dua kali tidak mengubah apa-apa lagi.

## Pesan kesalahan

Kesalahan ditunjukkan dengan nomor baris, kolom dan saran. Contoh:

```text
Baris 2  Tanda "-" tidak dikenal di baris not
         Perpanjangan ketuk ditulis dengan titik: 5 . . .
```

Peringatan (bukan kesalahan) muncul bila birama tidak sesuai `ketuk`, bila panjang suara
berbeda, atau bila suku kata lebih banyak dari not.
