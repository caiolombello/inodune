import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(
  new URL('../styles/tokens.css', import.meta.url),
  'utf8',
);
const block = (selector: string) => {
  const start = css.indexOf(`${selector} {`);
  expect(start).toBeGreaterThan(-1);
  const body = css.slice(start, css.indexOf('}', start));
  return body
    .split('\n')
    .slice(1)
    .map((line) => line.trim())
    .filter(Boolean)
    .sort();
};

describe('design tokens', () => {
  it('keeps the manual and the system dark palettes identical', () => {
    expect(block(":root:not([data-theme='light'])")).toEqual(
      block(":root[data-theme='dark']"),
    );
  });
  it('defines every token the dark palette overrides in the light root', () => {
    const light = new Set(
      [
        ...css
          .slice(0, css.indexOf('/* Dark palette'))
          .matchAll(/(--[a-z0-9-]+):/g),
      ].map((m) => m[1]),
    );
    for (const line of block(":root[data-theme='dark']")) {
      const name = line.split(':')[0];
      if (name.startsWith('--')) expect(light.has(name), name).toBe(true);
    }
  });
});
