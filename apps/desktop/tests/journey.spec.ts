import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
async function scan(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page
    .getByRole('button', { name: 'Iniciar análise de exemplo' })
    .click();
  await expect(
    page.getByRole('button', { name: 'Revisar candidatos' }),
  ).toBeEnabled();
}
test('complete journey: protect, explain, review, simulate and report with no real recovery', async ({
  page,
}) => {
  await scan(page);
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await expect(page.getByRole('checkbox')).toHaveCount(3);
  await expect(
    page.getByRole('checkbox', { name: 'Selecionar Build Rust' }),
  ).not.toBeChecked();
  await expect(
    page.getByRole('checkbox', { name: 'Selecionar Miniaturas' }),
  ).toBeChecked();
  await page
    .getByRole('button', { name: 'Por que apareceu: Sessões de agentes' })
    .click();
  await expect(page.getByRole('dialog')).toContainText(
    'Todas as sessões, JSONL e transcripts.',
  );
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('button', { name: 'Por que apareceu: Sessões de agentes' }),
  ).toBeFocused();
  await page.getByRole('button', { name: 'Revisar impacto' }).click();
  await expect(page.getByText('0 B na mesma unidade')).toBeVisible();
  await page.getByRole('button', { name: 'Revisar confirmação' }).click();
  await expect(
    page.getByRole('button', { name: 'Confirmar simulação' }),
  ).toBeDisabled();
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Confirmar simulação' }).click();
  await expect(
    page.getByRole('heading', { name: 'Simulação concluída.' }),
  ).toBeVisible();
  await expect(page.getByRole('heading', { level: 1 })).toBeFocused();
  await expect(
    page.getByText('Simulado · sem alteração', { exact: true }),
  ).toHaveCount(2);
  await expect(page.getByText('Não medido', { exact: true })).toBeVisible();
});
test('pause and cancel never suggests incomplete results', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await page
    .getByRole('button', { name: 'Analisar exemplo', exact: true })
    .click();
  await page.getByRole('button', { name: 'Pausar', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Análise pausada' }),
  ).toBeVisible();
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${test.info().project.name}-scan-paused.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(
    page.getByText('Análise cancelada', { exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Desenvolvimento', exact: true })
    .click();
  await expect(
    page.getByText('0 selecionados no total', { exact: false }),
  ).toBeVisible();
});
test('partial execution preserves failures and changed items', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await page.getByLabel('Cenário de demonstração').selectOption('partial');
  await page.getByRole('button', { name: 'Analisar cenário' }).click();
  await expect(
    page.getByRole('button', { name: 'Revisar candidatos' }),
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await page.getByRole('checkbox', { name: 'Selecionar Build Rust' }).check();
  await page.getByRole('button', { name: 'Revisar impacto' }).click();
  await page
    .getByRole('radio', { name: 'Exclusão permanente', exact: false })
    .check();
  await page.getByRole('button', { name: 'Revisar confirmação' }).click();
  await expect(page.getByRole('dialog')).toContainText('não pode ser desfeita');
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Confirmar simulação' }).click();
  await expect(
    page.getByRole('heading', { name: 'Processando exemplos, item a item.' }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Simulação concluída.' }),
  ).not.toBeVisible();
  await expect(page.getByText('Falhou · permissão simulada')).toBeVisible();
  await expect(page.getByText('Pulado · item alterado')).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${test.info().project.name}-report-partial.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await expect(
    page.getByRole('heading', { name: 'Simulação com itens não concluídos.' }),
  ).toBeVisible();
});
test('empty and error states remain truthful across innovation screens', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  for (const scenario of ['empty', 'error']) {
    await page
      .getByRole('button', { name: 'Configurações', exact: true })
      .click();
    await page.getByLabel('Cenário de demonstração').selectOption(scenario);
    await page.getByRole('button', { name: 'Analisar cenário' }).click();
    if (scenario === 'error') {
      await expect(page.getByRole('alert')).toContainText('Permissão negada');
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.screenshot({
        path: `../../docs/evidence/automatic/renderer/screenshots/${test.info().project.name}-scan-error.png`,
        fullPage: true,
        animations: 'disabled',
      });
    } else {
      await expect(
        page.getByRole('button', { name: 'Analisar novamente' }),
      ).toBeEnabled();
      await expect(page.getByText('Nenhum item nesta seleção')).toBeVisible();
    }
    await page.getByRole('button', { name: 'Projetos', exact: true }).click();
    await expect(page.getByText('Nenhum projeto analisado')).toBeVisible();
    await page
      .getByRole('button', { name: 'Itens protegidos', exact: false })
      .first()
      .click();
    await expect(page.getByText('Nenhum item nesta seleção')).toBeVisible();
  }
});
test('screenshots, contrast, reflow, keyboard and reduced motion', async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await scan(page);
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${info.project.name}-overview-light.png`,
    fullPage: true,
    animations: 'disabled',
  });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.getByRole('button', { name: 'Alternar tema' }).click();
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${info.project.name}-overview-dark.png`,
    fullPage: true,
    animations: 'disabled',
  });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${info.project.name}-narrow-dark.png`,
    fullPage: true,
    animations: 'disabled',
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await page
    .getByRole('button', { name: 'Por que apareceu: Miniaturas' })
    .click();
  expect(
    await page
      .getByRole('dialog')
      .evaluate((el) => getComputedStyle(el).animationName),
  ).toBe('none');
  await page.keyboard.press('Escape');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await page.getByLabel('Reduzir animações', { exact: false }).check();
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  expect(
    await page.evaluate(() => document.documentElement.dataset.motion),
  ).toBe('reduce');
  await page.screenshot({
    path: `../../docs/evidence/automatic/renderer/screenshots/${info.project.name}-settings-narrow.png`,
    fullPage: true,
    animations: 'disabled',
  });
  await page.setViewportSize({ width: 320, height: 700 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test('cancellation between items keeps completed outcomes and report estimate frozen', async ({
  page,
}) => {
  await scan(page);
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await page.getByRole('button', { name: 'Revisar impacto' }).click();
  await page.getByRole('button', { name: 'Revisar confirmação' }).click();
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Confirmar simulação' }).click();
  await page.getByRole('button', { name: 'Cancelar entre itens' }).click();
  await expect(page.getByText('Cancelado · não processado')).toBeVisible();
  await expect(page.getByText('Simulado · sem alteração')).toBeVisible();
  await page.getByRole('button', { name: 'Voltar à revisão' }).click();
  await page.getByRole('button', { name: 'Desmarcar todos' }).click();
  await page.getByRole('button', { name: 'Histórico', exact: true }).click();
  await expect(page.getByText('480 MiB', { exact: true })).toBeVisible();
});
test('project drill-down, keyboard focus and local-only renderer', async ({
  page,
}) => {
  const remote: string[] = [];
  page.on('request', (req) => {
    if (new URL(req.url()).hostname !== '127.0.0.1') remote.push(req.url());
  });
  await scan(page);
  await page.getByRole('button', { name: 'Projetos', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1 })).toBeFocused();
  await page.getByRole('button', { name: 'Atlas API', exact: false }).click();
  await expect(
    page.getByRole('heading', { name: 'Atlas API', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Cache Python', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Build Rust', exact: true }),
  ).not.toBeVisible();
  await page
    .getByRole('button', { name: 'Por que apareceu: Cache Python' })
    .click();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.keyboard.press('Escape');
  expect(remote).toEqual([]);
});
