import { expect, test } from '@playwright/test';

test('report explains irreversible cleanup and preserves a copyable recovery location', async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: async () => {
          throw new DOMException('Clipboard unavailable', 'NotAllowedError');
        },
      },
    });
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          if (cmd === 'list_scopes') return [];
          if (cmd === 'list_history')
            return [
              {
                report_id: 'report-cccccccccccccccccccccccccccccccc',
                created_unix: 1_790_000_000,
                plan_id: 'plan-recovery-fixture',
                action: 'permanent',
                complete: false,
                cancelled: true,
                logical_bytes_processed: 32,
                files_count: 2,
                error: null,
              },
            ];
          if (cmd === 'get_history_report')
            return {
              schema_version: 1,
              plan_id: 'plan-recovery-fixture',
              action: 'permanent',
              items: [
                {
                  candidate_id: 'candidate-fixture',
                  complete: false,
                  files: [
                    {
                      display_path:
                        '/tmp/inodune-fixture/project/cache/item.bin',
                      status: 'removed',
                      logical_bytes: 16,
                      message: null,
                      preserved_at: null,
                      durability_confirmed: true,
                    },
                    {
                      display_path:
                        '/tmp/inodune-fixture/project/cache/changed.bin',
                      status: 'quarantined',
                      logical_bytes: 16,
                      message: 'Recovery item preserved.',
                      preserved_at:
                        '/tmp/inodune-fixture/state/quarantine/changed.bin',
                      durability_confirmed: false,
                    },
                  ],
                },
              ],
              volumes: [],
              complete: false,
              cancelled: true,
              logical_bytes_processed: 32,
              attributed_freed_bytes: null,
              measurement_note: 'Observed only.',
              warnings: [],
              history_path: null,
            };
          throw new Error(`Unexpected fixture command: ${cmd}`);
        },
      },
    });
  });

  await page.goto('/');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .first()
    .click();
  await page
    .getByLabel('Modo de análise', { exact: true })
    .selectOption('manual');
  await page.getByRole('button', { name: 'Histórico', exact: true }).click();
  await page.getByRole('button', { name: /Exclusão permanente/ }).click();

  await expect(page.getByRole('alert').first()).toContainText(
    'Execução cancelada',
  );
  await expect(page.getByText(/não pode desfazer/).first()).toBeVisible();
  await expect(
    page.getByText(/sem atribuição a esta operação/).first(),
  ).toBeVisible();
  await expect(
    page.getByText('/tmp/inodune-fixture/state/quarantine/changed.bin').first(),
  ).toBeVisible();
  await expect(
    page.getByText(/durabilidade não foi confirmada/i),
  ).toBeVisible();

  await page
    .getByRole('button', { name: 'Copiar caminho de recuperação' })
    .click();
  await expect(page.locator('.recovery').getByRole('alert')).toContainText(
    'Não foi possível copiar o caminho',
  );
  await page.screenshot({ path: '/tmp/inodune-recovery.png', fullPage: true });
});
