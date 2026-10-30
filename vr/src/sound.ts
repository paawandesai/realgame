// Tiny synthesised cues, so the build ships no audio files.
let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === 'suspended') void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

function tone(freq: number, start: number, dur: number, type: OscillatorType, gain = 0.12): void {
  const a = audio();
  if (!a) return;
  const t = a.currentTime + start;
  const o = a.createOscillator();
  const g = a.createGain();
  o.type = type;
  o.frequency.setValueAtTime(freq, t);
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(gain, t + 0.01);
  g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  o.connect(g).connect(a.destination);
  o.start(t);
  o.stop(t + dur + 0.05);
}

export const sound = {
  drop: () => tone(660, 0, 0.12, 'sine', 0.08),
  success: () => [523, 659, 784, 1046].forEach((f, i) => tone(f, i * 0.07, 0.35, 'triangle')),
  partial: () => [523, 659, 784].forEach((f, i) => tone(f, i * 0.08, 0.3, 'triangle', 0.09)),
  fizzle: () => {
    tone(220, 0, 0.25, 'sawtooth', 0.05);
    tone(165, 0.1, 0.3, 'sawtooth', 0.04);
  },
  click: () => tone(1200, 0, 0.05, 'square', 0.04),
};
