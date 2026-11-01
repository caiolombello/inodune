// Web Audio synthesis stays local: no downloads, microphone, files or plugins.
// Audio is opt-in, unlocked only by a user gesture, and never the only feedback.
// https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/resume
export type FeedbackTone = 'start' | 'ready' | 'attention' | 'preview';
const notes: Record<FeedbackTone, number[]> = {
  start: [330],
  ready: [440, 554.37],
  attention: [330, 293.66],
  preview: [440, 554.37],
};

export class SoundFeedback {
  private enabled = false;
  private context: AudioContext | null = null;
  private unlocked = false;
  private generation = 0;
  private lastToneAt = -Infinity;

  constructor(
    private readonly createContext: () => AudioContext = () =>
      new AudioContext(),
    private readonly isVisible: () => boolean = () => !document.hidden,
  ) {}

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
    if (!enabled) this.dispose();
  }

  async unlock(): Promise<boolean> {
    if (!this.enabled || !this.isVisible()) return false;
    const generation = this.generation;
    try {
      const context = (this.context ??= this.createContext());
      await context.resume();
      if (
        generation !== this.generation ||
        !this.enabled ||
        context !== this.context ||
        context.state !== 'running'
      )
        return false;
      this.unlocked = true;
      return true;
    } catch {
      this.unlocked = false;
      return false;
    }
  }

  play(tone: FeedbackTone): boolean {
    const context = this.context;
    if (
      !this.enabled ||
      !this.unlocked ||
      !this.isVisible() ||
      !context ||
      context.state !== 'running' ||
      context.currentTime - this.lastToneAt < 0.45
    )
      return false;
    try {
      this.lastToneAt = context.currentTime;
      for (const [index, frequency] of notes[tone].entries()) {
        const oscillator = context.createOscillator();
        const envelope = context.createGain();
        const at = context.currentTime + index * 0.09;
        oscillator.type = 'sine';
        oscillator.frequency.setValueAtTime(frequency, at);
        envelope.gain.setValueAtTime(0, at);
        envelope.gain.linearRampToValueAtTime(0.055, at + 0.012);
        envelope.gain.linearRampToValueAtTime(0, at + 0.16);
        oscillator.connect(envelope);
        envelope.connect(context.destination);
        oscillator.onended = () => {
          oscillator.disconnect();
          envelope.disconnect();
        };
        oscillator.start(at);
        oscillator.stop(at + 0.17);
      }
      return true;
    } catch {
      // Never fail a filesystem operation because the sound device vanished.
      return false;
    }
  }

  dispose() {
    this.generation += 1;
    this.unlocked = false;
    this.lastToneAt = -Infinity;
    const context = this.context;
    this.context = null;
    if (context) void context.close().catch(() => undefined);
  }
}
