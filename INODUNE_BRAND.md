# Inodune — identidade visual v0.2

## Conceito

**Inodune** é suave e técnico ao mesmo tempo. A marca remete a *inode* (estrutura básica de arquivos no Unix) e a uma paisagem com espaço para respirar. A proposta visual traduz recuperação de espaço por um **anel incompleto com um bloco livre**, sem usar o clichê de vassoura/lixeira/foguete.

- **Assinatura:** “Liberte espaço. Preserve o essencial.”
- **Tom de voz:** calmo, direto, verificável. “Pronto para revisar”, não “Seu PC está em perigo!” ou “Limpe 100%”.
- **Personalidade:** precisão de ferramenta profissional, simplicidade de aplicativo consumer.

## Paleta

| Uso | Token | HEX |
| --- | --- | --- |
| Texto principal | Ink | `#14202B` |
| Marca profunda | Deep Teal | `#164F52` |
| Ação principal | Teal | `#1F7A71` |
| Acento do bloco liberado | Mint | `#58D5B4` |
| Fundo claro | Canvas | `#F7FAF8` |
| Cards | Surface | `#FFFFFF` |
| Borda leve | Stroke | `#DDE6E3` |
| Texto secundário | Muted | `#64747C` |
| Atenção | Warning | `#AD6B17` |
| Risco | Danger | `#B54646` |

## Tipografia e composição

- Inter, com fallback para `system-ui`, respeitando licença OFL ao distribuir a fonte.
- Títulos em `600–700`; corpo `400–500`. Espaçamento base 4px, escala 4/8/12/16/24/32.
- Cards brancos de raio 16–20px, bordas leves, sombra mínima, ícones de linha consistente.
- Dark theme com fundo `#10191D`, superfícies `#18262B`, texto `#E8F1EE`; testar contraste.
- Nunca apresentar risco apenas por cor: ícone + label textual + descrição são obrigatórios.
- Priorizar hierarquia visual e microcopy sobre enfeites; os dados sensíveis/destrutivos não devem competir com movimento decorativo.

## Motion language

- Personalidade: **fluido, silencioso, preciso**. Nenhum bounce/brilho exagerado.
- Tempos-base: instantâneo 80–120 ms, rápido 140–180 ms, padrão 180–240 ms e transições maiores 260–320 ms.
- Usar transições curtas para foco/hover, expansão de cards e drawers, mudança de filtros, entrada do resultado e confirmações reais.
- O anel de armazenamento pode ter movimento sutil durante scan; usar progresso verdadeiro quando conhecido e estado indeterminado quando não for.
- **Nunca animar uma deleção como concluída antes da resposta do backend.** Sucesso e ganho real são estados que dependem de medição.
- `prefers-reduced-motion` e preferência manual reduzem/eliminam movimento; manter feedback textual e acessibilidade.
- Consultar especificação completa em [`UX_MOTION_INNOVATION.md`](UX_MOTION_INNOVATION.md), inclusive tokens, performance e critérios de aceite.

## Assets

- `assets/brand/inodune-logo.svg`: logo horizontal vetorial, uso no produto e no README.
- `assets/brand/inodune-icon.svg`: ícone vetorial, base para `.desktop`, app icon e favicon.
- `assets/brand/inodune-icon-512.png`: export PNG do SVG em 512 × 512.
- `inodune-ui-concept.png`: conceito de interface para inspiração, não screenshot de produto concluído e não material com números reais.

Os SVGs são as fontes canônicas. Para variações sobre fundos escuros, derivar cores das tokens; não aplicar brilho, gradientes ou blur à marca. Manter respiro equivalente a pelo menos 20% do diâmetro do ícone em lockups.

## Componentes de interface

1. Dashboard de uso com número claro e qualificadores (“estimado”, “já liberado”).
2. Cards categorizados: Desenvolvimento, Git, IA & Agentes, Sistema.
3. Lista revisável, com seleção, caminho, tamanho, origem, última atividade e risco.
4. Drawer “Por que este item?” explicando que dados poderão ser perdidos e como recriá-los.
5. Diálogo de confirmação que discrimina exclusão irreversível versus Lixeira.
6. Relatório pós-limpeza mostrando sucesso, falha, pulado e espaço efetivamente alterado.
7. Simulador de ganho e risco **antes** de executar, sem dupla contagem.
8. Project Lens, que agrupa candidatos por repositório, e Protection Ledger, que explica itens deliberadamente preservados.
9. Estados completos (hover, focus, pressed, disabled, loading, error), com motion reduzido e layout adaptativo.

## Diretriz para Claude Code

Não reproduzir cegamente números ou caminhos de exemplo do mockup. Construir UI real alimentada pelo core Rust. A elegância visual tem de sobreviver a resultados vazios, scans longos, erros parciais e labels em português. Exigir protótipo navegável e evidência visual por fase, não apenas screenshot bonito.
