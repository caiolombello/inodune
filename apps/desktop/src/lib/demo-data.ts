// Corpus e textos fictícios: só entram em builds de desenvolvimento e teste.
import corpus from '../../../../tests/fixtures/demo.json';
import type { Inventory } from './model';

const inventory = corpus as Inventory;
if (
  inventory.mode !== 'simulation' ||
  inventory.candidates.some(
    (c) =>
      !['low', 'review', 'protected'].includes(c.risk) ||
      c.supported_action !== 'none' ||
      !c.display_path.startsWith('/demo/'),
  )
)
  throw new Error('Invalid demo corpus');

/** Traduções en-US dos textos que só aparecem na demonstração. */
export const catalog: Record<string, string> = {
  'Protótipo navegável': 'Interactive demo',
  'Pastas e valores fictícios. Seus arquivos não são acessados.':
    'Sample folders and values. Your files are not accessed.',
  'O primeiro passo é entender o que ocupa espaço.':
    'The first step is understanding what uses space.',
  'Um pouco mais de respiro.': 'A little more breathing room.',
  'Valores do corpus sintético, sem medição do disco.':
    'Synthetic corpus values; disk usage was not measured.',
  'volume de exemplo': 'sample volume',
  'Limpeza real indisponível na demonstração.':
    'Real cleanup is unavailable in the demo.',
  'Volume fictício: 512 GiB totais, 382 GiB ocupados e 130 GiB livres':
    'Sample volume: 512 GiB total, 382 GiB used, and 130 GiB free',
};
export default inventory;
