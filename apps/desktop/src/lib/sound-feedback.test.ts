import { describe, expect, it, vi } from 'vitest';
import { SoundFeedback } from './sound-feedback';

function audioFixture() {
  const starts: number[] = [];
  const context = {
    state: 'running',
    currentTime: 10,
    destination: {},
    resume: vi.fn(async (): Promise<void> => undefined),
    close: vi.fn(async () => undefined),
    createOscillator: () => ({
      type: '',
      frequency: { setValueAtTime: vi.fn() },
      connect: vi.fn(),
      disconnect: vi.fn(),
      start: (time: number) => starts.push(time),
      stop: vi.fn(),
      onended: null,
    }),
    createGain: () => ({
      gain: {
        setValueAtTime: vi.fn(),
        linearRampToValueAtTime: vi.fn(),
      },
      connect: vi.fn(),
      disconnect: vi.fn(),
    }),
  };
  const create = vi.fn(() => context as unknown as AudioContext);
  return { starts, context, create };
}

describe('optional sound feedback', () => {
  it('does not initialize audio or play until explicitly enabled and unlocked', async () => {
    const fixture = audioFixture();
    const sound = new SoundFeedback(fixture.create, () => true);
    expect(await sound.unlock()).toBe(false);
    sound.play('ready');
    expect(fixture.create).not.toHaveBeenCalled();
    sound.setEnabled(true);
    sound.play('ready');
    expect(fixture.starts).toHaveLength(0);
    expect(await sound.unlock()).toBe(true);
    sound.play('ready');
    expect(fixture.starts.length).toBeGreaterThan(0);
  });

  it('drops background events instead of replaying them when focus returns', async () => {
    const fixture = audioFixture();
    let visible = true;
    const sound = new SoundFeedback(fixture.create, () => visible);
    sound.setEnabled(true);
    await sound.unlock();
    visible = false;
    sound.play('ready');
    visible = true;
    expect(fixture.starts).toHaveLength(0);
    sound.play('attention');
    expect(fixture.starts.length).toBeGreaterThan(0);
  });

  it('muting during a pending audio unlock cannot produce a late sound', async () => {
    const fixture = audioFixture();
    let finish: () => void = () => undefined;
    fixture.context.resume = vi.fn(
      () => new Promise<void>((resolve) => (finish = resolve)),
    );
    const sound = new SoundFeedback(fixture.create, () => true);
    sound.setEnabled(true);
    const pending = sound.unlock();
    sound.setEnabled(false);
    finish();
    expect(await pending).toBe(false);
    sound.play('ready');
    expect(fixture.starts).toHaveLength(0);
    expect(fixture.context.close).toHaveBeenCalledOnce();
  });

  it('audio unavailable is a recoverable result, never an exception in the scan', async () => {
    const sound = new SoundFeedback(
      () => {
        throw new Error('No audio device');
      },
      () => true,
    );
    sound.setEnabled(true);
    expect(await sound.unlock()).toBe(false);
    expect(() => sound.play('attention')).not.toThrow();
  });
});
