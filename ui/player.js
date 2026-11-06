// Pemutar sederhana dengan WebAudio. Daftar nada dihitung oleh inti Rust.
export class Player {
  constructor() {
    this.ctx = null;
    this.nodes = [];
    this.timer = 0;
    this.playing = false;
    this.onStep = null;
    this.onEnd = null;
  }

  play(timeline, onlyVoice) {
    this.stop();
    const notes = timeline.notes.filter((n) => onlyVoice == null || n.voice === onlyVoice);
    if (!notes.length) return false;
    const AC = window.AudioContext || window.webkitAudioContext;
    this.ctx = this.ctx || new AC();
    const ctx = this.ctx;
    if (ctx.state === 'suspended') ctx.resume();
    const spb = 60 / timeline.bpm;
    const t0 = ctx.currentTime + 0.12;
    const master = ctx.createGain();
    const voices = new Set(notes.map((n) => n.voice)).size;
    master.gain.value = 0.8 / Math.max(1, voices * 0.75);
    master.connect(ctx.destination);
    this.nodes.push(master);
    let end = t0;
    for (const n of notes) {
      const start = t0 + n.start * spb;
      const dur = Math.max(0.05, n.dur * spb);
      const osc = ctx.createOscillator();
      const g = ctx.createGain();
      osc.type = n.voice === 0 ? 'triangle' : 'sine';
      osc.frequency.value = 440 * Math.pow(2, (n.midi - 69) / 12);
      const peak = n.voice === 0 ? 0.5 : 0.32;
      g.gain.setValueAtTime(0, start);
      g.gain.linearRampToValueAtTime(peak, start + 0.02);
      g.gain.setValueAtTime(peak * 0.85, Math.max(start + 0.03, start + dur - 0.07));
      g.gain.linearRampToValueAtTime(0, start + dur - 0.01);
      osc.connect(g).connect(master);
      osc.start(start);
      osc.stop(start + dur);
      this.nodes.push(osc, g);
      end = Math.max(end, start + dur);
    }
    const sorted = notes.map((n) => ({ at: t0 + n.start * spb, until: t0 + (n.start + n.dur) * spb, n })).sort((a, b) => a.at - b.at);
    this.playing = true;
    const tick = () => {
      if (!this.playing) return;
      const now = ctx.currentTime;
      if (this.onStep) this.onStep(sorted.filter((x) => x.at <= now && x.until > now).map((x) => x.n));
      if (now > end + 0.05) { this.stop(); return; }
      this.timer = requestAnimationFrame(tick);
    };
    tick();
    return true;
  }

  stop() {
    const was = this.playing;
    this.playing = false;
    cancelAnimationFrame(this.timer);
    for (const n of this.nodes) {
      try { if (n.stop) n.stop(); } catch (e) { /* sudah berhenti */ }
      try { n.disconnect(); } catch (e) { /* abaikan */ }
    }
    this.nodes = [];
    if (was && this.onEnd) this.onEnd();
  }
}
