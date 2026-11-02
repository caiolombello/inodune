import { expect, test } from '@playwright/test';

test('simula antes de confirmar e envia somente o ID do plano na execução', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: { cmd: string; args: unknown }[] = [];
    Object.assign(window, {
      isTauri: true,
      __uninstallCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string, args: unknown) => {
          calls.push({ cmd, args });
          if (cmd === 'list_scopes') return [];
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
                      id: 'vendor-tool',
                      name: 'Ferramenta de teste',
                      version: '1',
                      size_bytes: 100,
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
              id: 'opaque-plan',
              items: [{ id: 'vendor-tool;1;amd64;installed', version: '1' }],
              expires_in_seconds: 120,
              steam_handoff: false,
            };
          if (cmd === 'uninstall_execute')
            return 'Transação concluída pelo gerenciador de pacotes.';
          throw new Error(`Comando inesperado: ${cmd}`);
        },
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Programas & Jogos', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Listar programas e jogos', exact: true })
    .click();
  await page
    .getByRole('button', {
      name: 'Desinstalar Ferramenta de teste',
      exact: true,
    })
    .click();
  const dialog = page.getByRole('heading', { name: 'Confirmar desinstalação' });
  await expect(dialog).toBeVisible();
  // G2: friendly name and version are the main text; the raw id is folded away.
  const plan = page.getByRole('dialog', { name: 'Confirmar desinstalação' });
  await expect(plan.getByText('vendor-tool', { exact: true })).toBeVisible();
  await expect(
    plan.getByText('vendor-tool;1;amd64;installed', { exact: true }),
  ).toBeHidden();
  await plan.getByText('Identificadores técnicos').click();
  await expect(
    plan.getByText('vendor-tool;1;amd64;installed', { exact: true }),
  ).toBeVisible();
  const before = await page.evaluate(
    () =>
      (
        window as unknown as {
          __uninstallCalls: { cmd: string; args: unknown }[];
        }
      ).__uninstallCalls,
  );
  expect(before.some((call) => call.cmd === 'uninstall_execute')).toBe(false);
  await plan.getByRole('button', { name: 'Desinstalar 1 pacote' }).click();
  await expect(
    page.getByText('Transação concluída pelo gerenciador de pacotes.'),
  ).toBeVisible();
  const calls = await page.evaluate(
    () =>
      (
        window as unknown as {
          __uninstallCalls: { cmd: string; args: unknown }[];
        }
      ).__uninstallCalls,
  );
  expect(calls.find((call) => call.cmd === 'uninstall_execute')?.args).toEqual({
    planId: 'opaque-plan',
  });
  await expect(
    page.getByRole('heading', { name: 'Confirmar desinstalação' }),
  ).toHaveCount(0);
});
