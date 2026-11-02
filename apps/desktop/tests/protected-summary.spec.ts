import { test, expect } from '@playwright/test';

test('protected-only cards show occupied space and the block reason', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page
    .getByRole('button', { name: 'Iniciar análise de exemplo' })
    .click();
  await expect(
    page.getByRole('button', { name: 'Revisar candidatos' }),
  ).toBeEnabled();
  const git = page.locator('.category-card', { hasText: 'Git Worktrees' });
  await expect(git).toContainText('itens protegidos');
  await expect(git).toContainText('ocupados');
  await expect(git).not.toContainText('Potencial lógico estimado');
  const agents = page.locator('.category-card', { hasText: 'IA & Agentes' });
  await expect(agents).toContainText('itens protegidos');
  await expect(agents).toContainText('ocupados');
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await page
    .getByRole('button', { name: 'Por que apareceu: Sessões de agentes' })
    .click();
  await expect(page.getByRole('dialog')).toContainText('Motivo do bloqueio');
  await expect(page.getByRole('dialog')).toContainText(
    'Todas as sessões, JSONL e transcripts.',
  );
  await expect(page.getByText('dividida em')).toHaveCount(0);
});
