import { expect, test } from '@playwright/test';

// WCAG 2.4.11: keyboard focus in the review list never ends up hidden behind
// the sticky selection bar. Synthetic IPC only.
test('focused rows stay above the sticky selection bar', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.addInitScript(() => {
    const candidates = Array.from({ length: 30 }, (_, i) => ({
      id: `c${i}`,
      title: `Cache ${i}`,
      display_path: `/home/user/.cache/tool-${i}`,
      detector_id: `tool.${i}`,
      detector_version: '1',
      scope_id: '/home/user',
      category: 'Desenvolvimento',
      project: 'Pasta pessoal',
      risk: 'low',
      logical_bytes: 1_000_000 + i,
      allocated_bytes_estimate: 1_000_000 + i,
      identity: { dev: 1, ino: i, mtime_unix: 1 },
      coverage_key: `1:${i}`,
      selected_by_default: true,
      reason: 'Cache',
      impact: 'Recriável',
      rebuild_hint: 'A ferramenta recria.',
      preserved: 'Código preservado',
      evidence: [],
      activity: 'mtime',
      supported_action: 'cache_contents',
    }));
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          if (cmd === 'list_scopes') return [];
          if (cmd === 'begin_automatic_scan') return 'job';
          if (cmd === 'poll_scan')
            return {
              job_id: 'job',
              status: 'done',
              progress: { visited: 30, candidates: 30, stage: 'Concluído' },
              error: null,
              result: {
                scan_id: 'scan',
                candidates,
                issues: [],
                stats: { visited: 30, skipped: 0, elapsed_ms: 1 },
                complete: true,
                cancelled: false,
                volumes: [],
              },
            };
          return null;
        },
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  await page
    .locator('aside nav')
    .getByRole('button', { name: 'Desenvolvimento' })
    .click();
  await expect(page.locator('.selection-bar')).toHaveCSS('position', 'sticky');
  await page.getByRole('textbox', { name: 'Buscar item ou caminho' }).focus();
  let checked = 0;
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press('Tab');
    const hidden = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      const bar = document.querySelector('.selection-bar');
      if (!el || el === document.body || !bar || bar.contains(el)) return null;
      const a = el.getBoundingClientRect();
      const s = bar.getBoundingClientRect();
      return (
        a.bottom > s.top + 1 &&
        a.top < s.bottom &&
        a.right > s.left &&
        a.left < s.right
      );
    });
    if (hidden === null) continue;
    checked++;
    expect(hidden, `focus stop ${i} is behind the selection bar`).toBe(false);
  }
  expect(checked).toBeGreaterThan(20);
});
