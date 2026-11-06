/* Pemutar sederhana memakai WebAudio. */
(function (root) {
  'use strict';

  const SEMI = [0, 0, 2, 4, 5, 7, 9, 11]; // indeks 1..7
  // Tenor & bas biasanya ditulis satu oktaf lebih tinggi dari bunyinya.
  const VOICE_SHIFT = { S: 0, A: 0, T: -12, B: -12 };

  /** Ubah satu suara menjadi daftar nada {midi, start, dur, ev}. */
  function voiceToNotes(voice, tonic) {
    const notes = [];
    let last = null;
    for (const ev of voice.events) {
      if (ev.type === 'note') {
        const midi = tonic + SEMI[ev.degree] + ev.acc + 12 * ev.oct + (VOICE_SHIFT[voice.name] || 0);
        // Ikatan: not sama di dalam legato digabung.
        if (last && last.slurOpen && (ev.inSlur || ev.slurEnd) && last.midi === midi && Math.abs(last.start + last.dur - ev.start) < 1e-6) {
          last.dur += ev.dur;
          last.evs.push(ev);
        } else {
          last = { midi, start: ev.start, dur: ev.dur, evs: [ev] };
          notes.push(last);
        }
        last.slurOpen = !!(ev.slurStart || ev.inSlur) && !ev.slurEnd;
        if (ev.fermata) last.dur += 1;
      } else if (ev.type === 'ext') {
        if (last && last.start + last.dur >= ev.start - 1e-6) { last.dur += ev.dur; last.evs.push(ev); }
      } else if (ev.type === 'rest') {
        last = null;
      }
    }
    return notes;
  }

  function Player() {
    this.ctx = null;
    this.nodes = [];
    this.playing = false;
    this.onStep = null;
    this.onEnd = null;
    this.timer = null;
  }

  Player.prototype.play = function (song, opts) {
    this.stop();
    opts = opts || {};
    const PM = root.PMParser;
    const tonic = 60 + PM.parseKey(song.meta.key); // Do berada di oktaf C4..B4
    const bpm = Math.max(30, Math.min(240, Number(song.meta.bpm) || 90));
    const spb = 60 / bpm;
    const voices = opts.voices ? song.voices.filter((v) => opts.voices.includes(v.name)) : song.voices;
    if (!voices.length) return;

    const AC = root.AudioContext || root.webkitAudioContext;
    if (!AC) return;
    this.ctx = this.ctx || new AC();
    const ctx = this.ctx;
    if (ctx.state === 'suspended') ctx.resume();
    const t0 = ctx.currentTime + 0.15;
    const master = ctx.createGain();
    master.gain.value = 0.9 / Math.max(1, voices.length * 0.8);
    master.connect(ctx.destination);
    this.nodes.push(master);

    const timeline = [];
    let endBeat = 0;
    voices.forEach((voice) => {
      const notes = voiceToNotes(voice, tonic);
      // Hitung pergeseran waktu akibat fermata (semua suara ikut tertahan).
      for (const n of notes) {
        const start = t0 + n.start * spb + fermataShift(song.voices[0], n.start) * spb;
        const dur = n.dur * spb;
        const osc = ctx.createOscillator();
        const g = ctx.createGain();
        osc.type = voice.name === 'S' ? 'triangle' : 'sine';
        osc.frequency.value = 440 * Math.pow(2, (n.midi - 69) / 12);
        g.gain.setValueAtTime(0, start);
        g.gain.linearRampToValueAtTime(voice.name === 'S' ? 0.5 : 0.35, start + 0.02);
        g.gain.setValueAtTime(voice.name === 'S' ? 0.45 : 0.3, start + Math.max(0.03, dur - 0.08));
        g.gain.linearRampToValueAtTime(0, start + dur - 0.01);
        osc.connect(g).connect(master);
        osc.start(start);
        osc.stop(start + dur);
        this.nodes.push(osc, g);
        timeline.push({ time: start, ev: n.evs[0], vi: song.voices.indexOf(voice) });
        endBeat = Math.max(endBeat, start + dur);
      }
    });
    timeline.sort((a, b) => a.time - b.time);
    this.playing = true;

    const tick = () => {
      if (!this.playing) return;
      const now = ctx.currentTime;
      const active = [];
      for (const item of timeline) if (item.time <= now) active[item.vi] = item.ev;
      if (this.onStep) this.onStep(active.filter(Boolean));
      if (now > endBeat + 0.05) { this.stop(); return; }
      this.timer = root.requestAnimationFrame(tick);
    };
    tick();
  };

  /** Fermata di melodi menambah 1 ketuk untuk semua yang datang sesudahnya. */
  function fermataShift(melody, beat) {
    let shift = 0;
    for (const ev of melody.events) {
      if (ev.fermata && ev.start + ev.dur <= beat + 1e-6) shift += 1;
    }
    return shift;
  }

  Player.prototype.stop = function () {
    const wasPlaying = this.playing;
    this.playing = false;
    if (this.timer) root.cancelAnimationFrame(this.timer);
    for (const n of this.nodes) {
      try { if (n.stop) n.stop(); } catch (e) { /* sudah berhenti */ }
      try { n.disconnect(); } catch (e) { /* abaikan */ }
    }
    this.nodes = [];
    if (wasPlaying && this.onEnd) this.onEnd();
  };

  root.PMPlayer = Player;
  root.PMPlayerUtil = { voiceToNotes };
})(typeof self !== 'undefined' ? self : this);
