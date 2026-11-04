import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import type { Candidate } from '../src/lib/model';
import { EVIDENCE } from './evidence';

const corpus = JSON.parse(
  readFileSync(
    new URL('../../../tests/fixtures/demo.json', import.meta.url),
    'utf8',
  ),
) as { candidates: Candidate[] };

const evidenceDirectory = `${EVIDENCE}/automatic/renderer`;

for (const count of [100, 1_000, 10_000]) {
  test(`F4 UI synthetic IPC list ${count}: bounded DOM, reflow, keyboard and frame sample`, async ({
    page,
    browserName,
  }) => {
    await page.setViewportSize({ width: 1440, height: 960 });
    const candidates = Array.from({ length: count }, (_, index) => ({
      ...corpus.candidates[2],
      id: 'fixture-' + index,
      title: 'Build de fixture ' + index,
      display_path: '/fixture/project-' + index + '/target',
      project: 'Projeto de benchmark',
      scope_id: 'fixture-scope',
      coverage_key: 'fixture-' + index,
      logical_bytes: index + 1,
    }));

    await page.addInitScript(
      ({ items }) => {
        Object.assign(window, {
          isTauri: true,
          __TAURI_INTERNALS__: {
            invoke: async (cmd: string, args: Record<string, unknown>) => {
              if (cmd === 'list_scopes')
                return [
                  {
                    id: 'fixture-scope',
                    label: 'Fixture IPC',
                    display_path: '/fixture',
                    kind: 'projects',
                  },
                ];
              if (cmd === 'begin_scan') {
                if (JSON.stringify(args.scopeIds) !== '["fixture-scope"]')
                  throw Error('Wrong root IDs');
                return 'fixture-job';
              }
              if (cmd === 'poll_scan')
                return {
                  job_id: 'fixture-job',
                  status: 'done',
                  progress: {
                    visited: items.length,
                    candidates: items.length,
                    stage: 'Fixture',
                  },
                  error: null,
                  result: {
                    scan_id: 'fixture-scan',
                    candidates: items,
                    issues: [],
                    stats: { visited: items.length, skipped: 0, elapsed_ms: 0 },
                    complete: true,
                    cancelled: false,
                    volumes: [],
                  },
                };
              throw Error('Unexpected command ' + cmd);
            },
          },
        });
      },
      { items: candidates },
    );

    await page.goto('/');
    await page
      .getByRole('button', { name: 'Configurações', exact: true })
      .first()
      .click();
    await page
      .getByLabel('Modo de análise', { exact: true })
      .selectOption('manual');
    await page
      .getByRole('button', { name: 'Visão geral', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Analisar pastas', exact: true }),
    ).toBeEnabled();
    const start = await page.evaluate(() => performance.now());
    await page
      .getByRole('button', { name: 'Analisar pastas', exact: true })
      .click();
    await page.getByRole('button', { name: 'Abrir a lista completa' }).click();
    await expect(
      page.getByRole('heading', { name: 'Revisão de itens', exact: true }),
    ).toBeVisible();

    const visibleRows = await page.locator('.item-row').count();
    const elapsed = (await page.evaluate(() => performance.now())) - start;
    const domCount = await page.locator('body *').count();
    expect(visibleRows).toBeLessThanOrEqual(count === 100 ? 100 : 60);

    let pageDownScrollTop: number | null = null;
    let paginationRows: number | null = null;
    let paginationStatus: string | null = null;
    if (count > 100) {
      const region = page.getByRole('region', {
        name: 'Lista virtualizada de itens; paginação disponível acima',
      });
      await region.focus();
      await page.keyboard.press('PageDown');
      await expect
        .poll(() => region.evaluate((el) => el.scrollTop))
        .toBeGreaterThan(0);
      pageDownScrollTop = await region.evaluate((el) => el.scrollTop);
      await page.getByRole('button', { name: 'Usar paginação' }).click();
      await expect(page.locator('.item-row')).toHaveCount(100);
      paginationRows = await page.locator('.item-row').count();
      await page.getByRole('button', { name: 'Próxima', exact: true }).click();
      const pageStatus = page
        .getByRole('status')
        .filter({ hasText: 'Página 2' });
      await expect(pageStatus).toBeVisible();
      paginationStatus = await pageStatus.innerText();
    }

    await page
      .getByLabel('Buscar item ou caminho')
      .fill('/fixture/project-' + (count - 1) + '/target');
    await expect(page.locator('.item-row')).toHaveCount(1);
    await expect(
      page.getByRole('button', {
        name: 'Build de fixture ' + (count - 1),
        exact: true,
      }),
    ).toBeVisible();
    const searchResultRows = await page.locator('.item-row').count();
    await page.getByLabel('Buscar item ou caminho').fill('');
    await expect(page.locator('.item-row').first()).toBeVisible();

    const viewportBeforeNarrow = page.viewportSize();
    await page.setViewportSize({ width: 420, height: 800 });
    const reflow = await page.evaluate(() => ({
      viewportWidth: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      bodyWidth: document.body.scrollWidth,
    }));
    expect(reflow.documentWidth).toBeLessThanOrEqual(reflow.viewportWidth);

    // Informational only: software/headless scheduling cannot prove a native
    // 60 fps target on representative desktop hardware.
    const frames = await page.evaluate(
      () =>
        new Promise<{
          sampleCount: number;
          meanIntervalMs: number;
          p95IntervalMs: number;
          intervalsOver16_7ms: number;
          intervalsOver50ms: number;
        }>((resolve) => {
          const intervals: number[] = [];
          let last = 0;
          const tick = (now: number) => {
            if (last) intervals.push(now - last);
            last = now;
            if (intervals.length < 90) requestAnimationFrame(tick);
            else {
              const sorted = [...intervals].sort((a, b) => a - b);
              resolve({
                sampleCount: intervals.length,
                meanIntervalMs:
                  intervals.reduce((sum, value) => sum + value, 0) /
                  intervals.length,
                p95IntervalMs: sorted[Math.floor(sorted.length * 0.95)],
                intervalsOver16_7ms: intervals.filter((value) => value > 16.7)
                  .length,
                intervalsOver50ms: intervals.filter((value) => value > 50)
                  .length,
              });
            }
          };
          requestAnimationFrame(tick);
        }),
    );

    if (count === 10_000) {
      await mkdir(evidenceDirectory, { recursive: true });
      await page.screenshot({
        path: `${evidenceDirectory}/ui-10000-review-420px.png`,
      });
    }
    const result = {
      count,
      browser: browserName,
      userAgent: await page.evaluate(() => navigator.userAgent),
      viewportBeforeNarrow,
      reflow,
      scanToReviewMilliseconds: Math.round(elapsed * 1000) / 1000,
      visibleRowsBeforeSearch: visibleRows,
      bodyDescendantCount: domCount,
      pageDownScrollTop,
      paginationRows,
      paginationStatus,
      searchResultRows,
      rowsAfterClearingSearch: await page.locator('.item-row').count(),
      reducedMotion: await page.evaluate(
        () => matchMedia('(prefers-reduced-motion: reduce)').matches,
      ),
      frameSampleAfterReflow: frames,
      note: 'Synthetic IPC renderer only; frame sample is informational and does not establish representative-device 60 fps.',
    };
    await mkdir(evidenceDirectory, { recursive: true });
    await writeFile(
      `${evidenceDirectory}/ui-${count}.json`,
      JSON.stringify(result, null, 2) + '\n',
      'utf8',
    );
  });
}
