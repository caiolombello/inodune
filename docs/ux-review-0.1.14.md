# Revisão de UX/UI — 0.1.14

## Método

- **Capturas:** feitas com Playwright em Chromium e WebKit (MiniBrowser WPE, o motor mais próximo do WebKitGTK do Tauri). Tamanhos 1440×960, 760×900 e 420×860, em tema claro e escuro.
- **Dados:** IPC do Tauri simulado com dados sintéticos de uma home fictícia (`/home/ana`, cerca de 40 itens, volume de 512 GiB). Nenhum dado real foi lido.
- **Revisão independente:** crítica da proposta e três rodadas de QA visual (GPT-6.1 Terra), mais três rodadas de revisão de código e segurança (GPT-6.1 Sol) sobre o diff, até não restar achado médio ou alto.
- **Auditorias da linha de base:** varredura axe (wcag2a/aa, 21aa, 22aa), contraste por token, mapa de arquitetura e verificação local completa.
- **Jornada nativa:** WebKitGTK real via tauri-driver, em Xvfb, somente com fixtures temporárias (`scripts/native-automatic-e2e.py`).

## Problemas encontrados (antes)

| Área | Problema | Categoria |
| --- | --- | --- |
| Todas | Exceção em `simulateBudget` dentro de `$derived` deixava a tela vazia com navegação inconsistente; só recarregar recuperava. | Robustez |
| Todas | Hover do botão primário com contraste de 1,1:1 (claro) e 1,26:1 (escuro), inclusive "Enviar N itens para a Lixeira". | Contraste, WCAG 1.4.3 |
| Todas | Bordas de busca e `select` com 1,27:1; `--warning` sem variante escura (3,62:1); anel do disco 2,74:1 contra o trilho no escuro; traço da marca quase invisível no escuro. | Contraste, tema |
| Todas | 64% do texto renderizado com 12 px ou menos, incluindo botões, títulos de item e o texto do consentimento. | Legibilidade |
| Todas | H1 com slogans ("Construa. Revise. Recomece.") e o mesmo eyebrow em cinco páginas; `<title>` só mudava em duas telas. | Orientação, WCAG 2.4.6 |
| Sidebar | "Programas & Jogos" acima da Visão geral e com o mesmo ícone de Sistema; textos de enchimento; nenhum estado da análise. | Navegação |
| Visão geral | Aviso âmbar de cobertura como primeiro elemento; número grande (potencial total) diferente do CTA (seleção); cards com estruturas diferentes; card que se erguia no hover sem ser clicável. | Hierarquia, affordance |
| Revisão | "Exige cuidado" cinza e sem ícone no lugar do risco real; o mesmo chevron para expandir e para detalhes; barra de ações no fim da lista. | Consistência |
| Projetos | Sem ordem; "1 protegidos"; jargão "sem contêineres sobrepostos". | Texto |
| Simulador | Dez linhas idênticas ("Bytecode Python" × 5…); "último scan". | Densidade |
| Relatório | Um cartão por arquivo, com a mesma frase de recuperação repetida e "Durabilidade confirmada: Sim" em todos; sem resumo por estado; variação de volume na cor de sucesso. | Hierarquia, honestidade |
| Histórico | Limpeza cancelada exibida como "Incompleto". | Honestidade |
| Estreito | Faixa de navegação cortada sem indicação de continuação. | Responsivo |
| CSS | Seletor global `nav` vazava para a paginação do relatório; três tokens referenciados sem definição. | Técnico |

## Decisões

- Ocupado e disponível são medidos; a seleção é uma estimativa lógica e fica fora do gráfico (ADR 0003). Um segmento "selecionado" no anel chegou a ser implementado e foi retirado após a revisão independente.
- Resultados do backend fora do contrato são recusados antes de entrar no estado; uma estimativa inválida bloqueia a limpeza (ADR 0002).
- Totais de exibição somam só tamanhos conhecidos e sem sobreposição. Desconhecido aparece como "Não medido", "≥ X" ou "N itens sem tamanho somado", nunca como 0 B.
- O risco real sempre aparece como badge com ícone. "Exige cuidado" é uma nota adicional; grupos mostram o risco mais cauteloso entre os membros.
- "Limpar com 1 clique" vira "Limpar lote seguro da análise", porque atua sobre o lote seguro global do backend, não sobre a seleção visível. O texto diz o destino e que a confirmação do sistema vem a seguir.
- A barra de seleção fica fixa em janelas a partir de 960×700. Um elemento da lista que recebe foco rola para acima dela (WCAG 2.4.11). `scroll-padding` sozinho não bastava: no WebKit, até 27 de 56 paradas de foco ficavam cobertas.
- O detalhe do item vira um painel lateral, mas continua sendo o mesmo `<dialog>` modal, com foco inicial, contenção, Escape e retorno.
- A lista da Visão geral continua recolhida por padrão (decisão da 0.1.8).

## Verificação

- `tests/robustness.spec.ts`: contrato IPC (duplicado, fracionário, acima do inteiro seguro, campo ausente), soma fora do limite e seleção sem estimativa bloqueadas, protegidos sem tamanho.
- `tests/focus-not-obscured.spec.ts`: 40 paradas de Tab em 1280×720 sem foco atrás da barra. O teste falha quando a correção é removida.
- `tests/grouped-rows.spec.ts`: grupos expandidos sobrevivem a fechar e reabrir a lista da Visão geral, que só é montada quando aberta.
- `src/lib/model.test.ts` (contrato completo, `trySimulateBudget`, `measuredSum`) e `src/lib/tokens.test.ts` (paletas escuras idênticas).
- Os testes existentes foram atualizados só onde a mudança foi intencional:
  - títulos funcionais em `automatic-scan`, `ux-review` e `i18n`;
  - o papel `switch` em `scan-experience`;
  - o texto "Análise cancelada" restrito ao `main` em `journey`;
  - o destino das evidências (`tests/evidence.ts`) em `journey`, `cleanup`, `scan-experience`, `virtual-list` e `virtual-list-f4`.

  Nenhuma verificação de segurança foi removida. O revisor técnico conferiu isso e comparou as 33 asserções do harness nativo antes e depois.
- Jornada nativa WebKitGTK com fixtures: PASS, e o harness foi atualizado. Já antes desta versão ele esperava 3 itens selecionáveis (o `target/` do Cargo passou a ser revisável em `e2871d2`), procurava um botão que só existe na demonstração e usava um título antigo de fixture.

## Limites

As capturas e o axe não substituem leitor de tela real, zoom de 200% numa sessão interativa, Wayland/GNOME/KDE real nem participantes. A jornada nativa roda em Xvfb com software rendering. O contraste foi medido por token e por estilo computado no Chromium; o WebKitGTK nativo não foi medido.
