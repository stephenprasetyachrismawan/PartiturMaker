/* Contoh lagu. Melodi dan syair dibuat khusus untuk contoh (bebas dipakai). */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.PMExamples = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const satb = `# Contoh 4 suara (SATB), gaya buku nyanyian jemaat
nomor: 1
judul: Muliakan Nama-Nya
subjudul: Mazmur 100
nada: Do = G
birama: 4/4
tempo: Khidmat
bpm: 84
syair: Contoh PartiturMaker
lagu: Contoh PartiturMaker
catatan: Contoh lagu bawaan PartiturMaker - bebas dipakai.

S: 1 1 3 5 | 6 5 3 . | 6, 1 2 3 | 2 1 2 . |
S: 5 5 6 1' | 7 6 5 . | 3 4 5 (4/ 3/) | 2 2 1 . |.

A: 5, 6, 1 3 | 4 3 1 . | 4, 5, 7, 1 | 7, 1 7, . |
A: 3 3 4 4 | 5 4 3 . | 1 1 3 1 | 7, 7, 1 . |.

T: 3 4 5 5 | 1' 1' 5 . | 2 3 5 5 | 4 3 2 . |
T: 1 3 1 1 | 2 1 1 . | 5 6 5 6 | 5 4 3 . |.

B: 1 4, 1 1 | 4, 1 1 . | 2, 1 5, 1 | 5, 1 5, . |
B: 1 3, 4, 4, | 5, 4, 1 . | 1 4, 3, 4, | 5, 5, 1 . |.

1: Mu-li-a-kan na-ma-Nya, ha-i u-mat se-mu-a!
1: Ka-sih-Nya t'lah me-nya-ta, ter-pan-car di du-ni-a.
2: Na-nyi-kan la-gu ba-ru de-ngan se-ge-nap ha-ti;
2: Tu-han Al-lah Pe-no-long ki-ni dan se-la-ma-nya.
`;

  const melodi = `# Contoh melodi saja, dengan ketuk pecahan, kres, istirahat dan pengulangan
nomor: 2
judul: Fajar Pagi
nada: Do = D
birama: 3/4
tempo: Riang
bpm: 104
syair: Contoh PartiturMaker
lagu: Contoh PartiturMaker

S: |: 1 . 3 | 5 . (6/ 5/) | 4 3 2 | 3 . 0 | \\
S: 5 . 6/ 7/ | 1' . 5 | 4#/ 5/ 6 4# | 5 . 0 | \\
S: 3/ 4/ 5 1' | 6 5 3 | 4/ 3/ 2/ 1/ 7, | 1 . 0^ :|

1: Fa-jar pa-gi t'lah me-nying-sing,
1: bu-rung ber-ki-cau, ber-nya-nyi ri-ang,
1: ma-ri pu-ji Tu-han de-ngan sa-tu su-a-ra.
`;

  return [
    { id: 'satb', name: 'Contoh 4 suara (SATB)', text: satb },
    { id: 'melodi', name: 'Contoh melodi + lirik', text: melodi },
  ];
});
