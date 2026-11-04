import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mkdir } from 'node:fs/promises';
import { writeFile } from 'node:fs/promises';
import { EVIDENCE } from './evidence';

test.use({ video: { mode: 'on', size: { width: 1440, height: 1000 } } });

test.afterEach(async ({ page }, info) => {
  if (!info.title.startsWith('records scanner motion')) return;
  const video = page.video();
  await page.close();
  if (video)
    await video.saveAs(`${EVIDENCE}/launch/scanner/fixture-motion.webm`);
});

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    Object.assign(window, {
      isTauri: true,
      __scanDone: false,
      __scanActions: [] as string[],
      __TAURI_INTERNALS__: {
        invoke: async (command: string, args: { action?: string } = {}) => {
          if (command === 'list_scopes') return [];
          if (command === 'begin_automatic_scan') return 'fixture-live-scan';
          if (command === 'control_scan') {
            (
              window as unknown as { __scanActions: string[] }
            ).__scanActions.push(args.action ?? '');
            return;
          }
          if (command === 'poll_scan') {
            const done = (window as unknown as { __scanDone: boolean })
              .__scanDone;
            return {
              job_id: 'fixture-live-scan',
              status: done ? 'done' : 'running',
              error: null,
              progress: {
                visited: 314,
                candidates: 2,
                stage: 'Verificando worktrees Git em sandbox somente leitura',
                recent_findings: [
                  {
                    id: 'fixture-cache',
                    title: 'Miniaturas de fixture',
                    category: 'Sistema',
                    risk: 'low',
                    logical_bytes: 4096,
                  },
                  {
                    id: 'fixture-memory',
                    title: 'Memória de agente preservada',
                    category: 'IA & Agentes',
                    risk: 'protected',
                    logical_bytes: 8192,
                  },
                ],
              },
              result: done
                ? {
                    scan_id: 'fixture-live-result',
                    candidates: [],
                    issues: [],
                    stats: { visited: 314, skipped: 0, elapsed_ms: 200 },
                    complete: true,
                    cancelled: false,
                    volumes: [],
                  }
                : null,
            };
          }
          throw new Error(`Unexpected fixture command ${command}`);
        },
      },
    });
  });
});

test('keeps audio opt-in and waits for a confirmed backend result before playing a finish cue', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const audio = { contexts: 0, notes: 0 };
    Object.assign(window, { __fixtureAudio: audio });
    Object.defineProperty(window, 'AudioContext', {
      value: class {
        state = 'running';
        currentTime = 10;
        destination = {};
        constructor() {
          audio.contexts += 1;
        }
        async resume() {}
        async close() {}
        createOscillator() {
          return {
            frequency: { setValueAtTime() {} },
            connect() {},
            disconnect() {},
            start() {
              audio.notes += 1;
            },
            stop() {},
          };
        }
        createGain() {
          return {
            gain: { setValueAtTime() {}, linearRampToValueAtTime() {} },
            connect() {},
            disconnect() {},
          };
        }
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  const enabled = page.getByRole('switch', { name: /Efeitos sonoros/ });
  await expect(enabled).not.toBeChecked();
  const snapshot = () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __fixtureAudio: { contexts: number; notes: number };
          }
        ).__fixtureAudio,
    );
  expect(await snapshot()).toEqual({ contexts: 0, notes: 0 });
  await enabled.check();
  await page
    .getByRole('button', { name: 'Analisar computador', exact: true })
    .click();
  await expect(
    page.getByText('Miniaturas de fixture', { exact: true }),
  ).toBeVisible();
  expect(await snapshot()).toEqual({ contexts: 1, notes: 0 });
  await page.evaluate(() => {
    (window as unknown as { __scanDone: boolean }).__scanDone = true;
  });
  await expect.poll(async () => (await snapshot()).notes).toBe(2);
  await page.reload();
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await expect(enabled).toBeChecked();
  expect(await snapshot()).toEqual({ contexts: 0, notes: 0 });
});

test('shows backend discoveries and protection during a genuinely indeterminate scan', async ({
  page,
}) => {
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  const scan = page.getByRole('region', { name: 'Análise em andamento' });
  await expect(
    scan.getByText('Miniaturas de fixture', { exact: true }),
  ).toBeVisible();
  await expect(
    scan.getByText('Memória de agente preservada', { exact: true }),
  ).toBeVisible();
  await expect(scan.getByText('314', { exact: true })).toBeVisible();
  await expect(
    scan.getByText('Verificando worktrees Git', { exact: true }),
  ).toBeVisible();
  await expect(scan.getByRole('progressbar')).toHaveCount(0);
  await expect(
    scan.getByText(
      'Total ainda desconhecido. Sem estimativa de tempo restante.',
    ),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Revisar candidatos', exact: true }),
  ).toHaveCount(0);
  const a11y = await new AxeBuilder({ page })
    .include('[data-scan-experience]')
    .analyze();
  expect(a11y.violations).toEqual([]);

  await scan.getByRole('button', { name: 'Pausar', exact: true }).click();
  await expect(
    scan.getByRole('heading', { name: 'Análise pausada' }),
  ).toBeVisible();
  const running = await scan.evaluate((element) =>
    element
      .getAnimations({ subtree: true })
      .some(
        (animation) =>
          animation.effect?.getTiming().iterations === Infinity &&
          animation.playState === 'running',
      ),
  );
  expect(running).toBe(false);
  await scan.getByRole('button', { name: 'Continuar', exact: true }).click();
  await expect(
    scan.getByRole('heading', { name: 'Analisando volumes locais' }),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as unknown as { __scanDone: boolean }).__scanDone = true;
  });
  await expect(scan).toHaveCount(0);
});

test('keeps motion optional, reflows at 390px and respects the operating system', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  const scan = page.getByRole('region', { name: 'Análise em andamento' });
  await expect(
    scan.getByText('Miniaturas de fixture', { exact: true }),
  ).toBeVisible();
  expect(
    await scan.evaluate(
      (element) => element.getAnimations({ subtree: true }).length,
    ),
  ).toBe(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await scan.getByRole('button', { name: 'Reduzir movimento' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
});

test('captures local scanner motion states without scanning any user data', async ({
  page,
}) => {
  await mkdir(`${EVIDENCE}/launch/scanner`, { recursive: true });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  await expect(
    page.getByText('Miniaturas de fixture', { exact: true }),
  ).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  expect(await page.evaluate(() => document.fonts.check('16px Inter'))).toBe(
    true,
  );
  await page.screenshot({
    path: `${EVIDENCE}/launch/scanner/light-running.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Alternar tema' }).click();
  await page.screenshot({
    path: `${EVIDENCE}/launch/scanner/dark-running.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Pausar', exact: true }).click();
  await page.screenshot({
    path: `${EVIDENCE}/launch/scanner/dark-paused.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: `${EVIDENCE}/launch/scanner/narrow-paused.png`,
    fullPage: true,
    animations: 'disabled',
  });
});

test('records scanner motion and its reduced alternative with only fixture IPC', async ({
  page,
}) => {
  await mkdir(`${EVIDENCE}/launch/scanner`, { recursive: true });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  const scan = page.getByRole('region', { name: 'Análise em andamento' });
  await expect(
    scan.getByText('Miniaturas de fixture', { exact: true }),
  ).toBeVisible();
  const intervals = await page.evaluate(
    () =>
      new Promise<number[]>((resolve) => {
        const samples: number[] = [];
        let previous = 0;
        const sample = (timestamp: number) => {
          if (previous) samples.push(timestamp - previous);
          previous = timestamp;
          if (samples.length < 90) requestAnimationFrame(sample);
          else resolve(samples);
        };
        requestAnimationFrame(sample);
      }),
  );
  expect(
    await scan.evaluate((element) =>
      element
        .getAnimations({ subtree: true })
        .some(
          (animation) =>
            animation.effect?.getTiming().iterations === Infinity &&
            animation.playState === 'running',
        ),
    ),
  ).toBe(true);
  await scan.getByRole('button', { name: 'Reduzir movimento' }).click();
  expect(
    await scan.evaluate(
      (element) =>
        element
          .getAnimations({ subtree: true })
          .filter(
            (animation) =>
              animation.effect?.getTiming().iterations === Infinity,
          ).length,
    ),
  ).toBe(0);
  await page.screenshot({
    path: `${EVIDENCE}/launch/scanner/reduced-running.png`,
    animations: 'disabled',
  });
  await writeFile(
    `${EVIDENCE}/launch/scanner/frame-sample.json`,
    JSON.stringify(
      {
        environment:
          'Playwright Chromium headless; fixture IPC; software observation only',
        intervals_ms: intervals,
        mean_ms: intervals.reduce((a, b) => a + b, 0) / intervals.length,
        maximum_ms: Math.max(...intervals),
        over_33ms: intervals.filter((value) => value > 33).length,
        limitation:
          'Does not certify native GPU, representative hardware or 60fps.',
      },
      null,
      2,
    ) + '\n',
  );
});
