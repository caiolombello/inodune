import { expect, test } from '@playwright/test';

const shot = (name: string) => `../../target/screenshots/0.1.7/${name}.png`;

test('overview clean flow and uninstall button', async ({ page }) => {
  await page.addInitScript(() => {
    const candidate = (
      id: string,
      title: string,
      category: string,
      bytes: number,
      risk: string,
      detector: string,
      action: string,
      reason: string,
    ) => ({
      id,
      title,
      display_path: `/fixture/${id}`,
      detector_id: detector,
      detector_version: '1',
      scope_id: 'home',
      category,
      project: 'Fixture',
      risk,
      logical_bytes: bytes,
      allocated_bytes_estimate: bytes,
      identity: { dev: 1, ino: bytes, mtime_unix: 1 },
      coverage_key: `${id}-key`,
      selected_by_default: risk === 'low' && detector !== 'node.modules',
      reason,
      impact: 'Recriável',
      rebuild_hint: 'A ferramenta recria.',
      preserved: 'Código preservado',
      evidence: [],
      activity: 'mtime',
      supported_action: action,
    });
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string, args?: { candidateIds?: string[] }) => {
          if (cmd === 'list_scopes') return [];
          if (cmd === 'begin_automatic_scan') return 'job-1';
          if (cmd === 'poll_scan')
            return {
              job_id: 'job-1',
              status: 'done',
              progress: { visited: 4, candidates: 4, stage: 'Concluído' },
              error: null,
              result: {
                scan_id: 'scan-1',
                candidates: [
                  candidate(
                    'npm',
                    'Cache npm',
                    'Desenvolvimento',
                    500_000_000,
                    'low',
                    'npm.cache',
                    'cache_contents',
                    'Cache recriável',
                  ),
                  candidate(
                    'thumbs',
                    'Miniaturas',
                    'Sistema',
                    100_000_000,
                    'low',
                    'thumbnails',
                    'cache_contents',
                    'Miniaturas recriáveis',
                  ),
                  candidate(
                    'node',
                    'Dependências Node',
                    'Desenvolvimento',
                    200_000_000,
                    'low',
                    'node.modules',
                    'cache_contents',
                    'node_modules recriável',
                  ),
                  candidate(
                    'provider',
                    'Provider Terraform',
                    'Desenvolvimento',
                    80_000_000,
                    'review',
                    'terraform.provider-package',
                    'reviewed_cache_contents',
                    'Versão referenciada por lock; o Terraform volta a baixá-la no init.',
                  ),
                  candidate(
                    'secret',
                    'Estado protegido',
                    'Desenvolvimento',
                    1000,
                    'protected',
                    'protected.state',
                    'none',
                    'Credencial',
                  ),
                ],
                issues: [],
                stats: {
                  visited: 4,
                  skipped: 0,
                  elapsed_ms: 1,
                  skip_reasons: {},
                },
                complete: true,
                cancelled: false,
                volumes: [
                  {
                    scope_path: '/fixture',
                    device: 1,
                    total_bytes: 1000,
                    available_bytes: 400,
                  },
                ],
              },
            };
          if (cmd === 'prepare_cleanup')
            return {
              id: 'plan-1',
              hash: 'hash-1',
              snapshot_id: 'scan-1',
              action: 'trash',
              expires_in_seconds: 120,
              items: (args?.candidateIds ?? ['npm']).map((id) => ({
                candidate_id: id,
                display_path: `/fixture/${id}`,
                logical_bytes: 500_000_000,
                allocated_bytes_estimate: 500_000_000,
              })),
              logical_bytes_estimate: 500_000_000,
              allocated_bytes_estimate: 500_000_000,
              review_required: false,
              inventory_complete: true,
            };
          if (cmd === 'begin_cleanup') return 'clean-1';
          if (cmd === 'poll_cleanup')
            return {
              status: 'done',
              progress: {
                completed_files: 1,
                total_files: 1,
                completed_items: 1,
                total_items: 1,
              },
              error: null,
              report: {
                schema_version: 1,
                plan_id: 'plan-1',
                action: 'trash',
                items: [],
                volumes: [],
                complete: true,
                cancelled: false,
                logical_bytes_processed: 400_000_000,
                attributed_freed_bytes: null,
                measurement_note: 'Variação observada no filesystem.',
                warnings: [],
                history_path: null,
              },
            };
          if (cmd === 'list_installed_apps')
            return {
              sources: [
                {
                  source: 'apt',
                  status: 'ok',
                  warnings: [],
                  items: [
                    {
                      source: 'apt',
                      id: 'hello',
                      name: 'hello',
                      version: '2',
                      size_bytes: 1024,
                      install_date: null,
                      protected: false,
                      notes: [],
                      signals: [],
                    },
                  ],
                },
              ],
            };
          if (cmd === 'uninstall_prepare')
            return {
              id: 'plan-apps',
              items: [{ id: 'hello', version: '2' }],
              expires_in_seconds: 120,
              steam_handoff: false,
            };
          if (cmd === 'uninstall_execute')
            return 'Transação concluída pelo gerenciador de pacotes.';
          return null;
        },
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador', exact: true })
    .first()
    .click();
  await expect(page.getByRole('button', { name: /Limpar/ }).first()).toBeVisible();
  await expect(page.getByText('Cache npm')).toBeHidden();
  await expect(page.getByText('Item / origem')).toHaveCount(0);
  await page.screenshot({ path: shot('overview'), fullPage: true });
  await page.getByText('Revisar itens', { exact: true }).click();
  await expect(page.getByRole('heading', { name: /Protegidos/ })).toBeVisible();
  await expect(page.getByText('Credencial')).toBeVisible();
  await expect(page.getByText('Exige cuidado').first()).toBeVisible();
  const sidebar = await page.locator('.sidebar').boundingBox();
  const skip = await page.locator('.skip-link').boundingBox();
  const skipFocused = await page.evaluate(
    () => document.activeElement?.classList.contains('skip-link') ?? false,
  );
  expect(sidebar?.x).toBe(0);
  expect(sidebar?.y).toBe(0);
  expect(skipFocused).toBe(false);
  expect((skip?.y ?? 0) < 0).toBeTruthy();
  await page.locator('.review-drawer').scrollIntoViewIfNeeded();
  await page.screenshot({ path: shot('review-open'), fullPage: false });
  const npm = page.getByRole('checkbox').first();
  await npm.uncheck();
  await page
    .getByRole('button', { name: /Limpar/ })
    .first()
    .click();
  await expect(
    page.getByRole('dialog', { name: 'Confirmar limpeza' }),
  ).toBeVisible();
  await page.screenshot({ path: shot('confirm'), fullPage: true });
  await page
    .getByRole('button', { name: 'Confirmar limpeza', exact: true })
    .click();
  await expect(
    page.getByText(/Variação observada|Execução|relatório|Concluído/i).first(),
  ).toBeVisible();
  await page.screenshot({ path: shot('summary'), fullPage: true });
  await page
    .getByRole('button', { name: 'Programas & Jogos', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Listar programas e jogos', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Desinstalar', exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: shot('apps-uninstall'), fullPage: true });
});
