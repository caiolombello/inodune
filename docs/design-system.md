# Design system

Direção herdada da marca: clean, silenciosa e precisa. Fonte local Inter se instalada, system-ui como fallback; sem font CDN. Grid 4px, espaçamentos 4/8/12/16/24/32, corpo 14–16px, títulos 24/30px, números tabulares. Cards 18px e controles 10px; sombras discretas. SVGs originais importados diretamente de assets/brand, sem redesenho.

CSS em apps/desktop/src/styles/tokens.css: ink #14202B, deepTeal #164F52, teal #1F7A71, mint #58D5B4, canvas #F7FAF8, surface #FFFFFF, stroke #DDE6E3, muted #64747C, warning #AD6B17, danger #B54646. Cores semânticas sobre fundos claros usam variantes escuras para AA; dark #10191D/#18262B/#E8F1EE e mint como ação com texto escuro. Preferência automática do sistema + escolha explícita. Alto contraste usa bordas/forced-colors.

Primitivos: button nativo (primary/secondary/quiet), badge de risco com ícone/texto, card categoria, lista selecionável, anel com legenda, dialog nativo, campo de busca e status anunciado. Estados hover/focus-visible/active/disabled/loading/error; sem sucesso optimista. Catálogo interno exercita loading/empty/error/partial/paused e controles.

Metas: contraste texto ≥4.5:1, UI/foco ≥3:1; alvos ≥36px (44px em viewport estreito), foco visível; leitor de tela e teclado. Auditoria automática é evidência parcial, não certificação WCAG. Listas grandes/virtualização pertencem à F1/F4; corpus limitado da F0 não demonstra escala.


F4: idioma PT-BR/EN-US acompanha valores numéricos e datas, preservando caminhos e nomes definidos pelo usuário. Histórico usa os mesmos tokens de superfície/stroke/texto/erro/focus; lista de outcomes pagina 100 arquivos e não cria linhas fictícias. Exclusões têm estados editing/saving/error/disabled e feedback após backend. Screenshots desktop/reflow estão em `evidence/f4`; controles nativos seguem o tema/idioma do sistema além do título localizado do app.


0.1.2 inclui Inter Variable 4.1 localmente, sem CDN, com créditos/licença nas configurações. Fonte oficial: [rsms/inter v4.1](https://github.com/rsms/inter/releases/tag/v4.1); [SIL OFL](https://raw.githubusercontent.com/rsms/inter/v4.1/LICENSE.txt). Arquivos em assets/fonts; WOFF2 SHA-256 693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3. O novo scanner usa as superfícies, raios, tipografia e tokens de motion existentes; revela classificação provisória sem transformar inventário protegido em ganho. Guia de recuperação mantém avisos persistentes, caminho selecionável e falha explícita de clipboard.

## 0.1.14 — escala, contraste e regras de gráfico

**Tipografia** (`tokens.css`): `--text-xs` 12 (eyebrow, badge, legenda), `--text-sm` 13 (metadado, caminho, legenda de gráfico), `--text-md` 14 (corpo, botões, campos, título de item), `--text-lg` 16 (título de seção), `--text-xl` 20, `--text-2xl` 24 (métricas de relatório, título do detalhe), `--text-metric` 28 (número de card), `--text-3xl` clamp(26–30) para `h1`, `--text-display` clamp(32–40) para o número principal. Rótulo acionável nunca abaixo de 14 px. Tracking 0 no corpo; títulos e números grandes em −0,015 a −0,02em. A Inter variável tem eixo `opsz` 14–32, que já aperta o espaçamento no display; não somar tracking negativo forte.

**Cor:** `--muted` claro escurecido para #5e6e76, porque o #64747c da marca ficava abaixo de 4,5:1 sobre fundos tingidos; `--muted` escuro #97abab, separado de `--secondary-text`. `--control-border` (#7f9196 claro / #62777c escuro) dá 3:1 aos limites de campos e botões secundários; `--stroke` fica para divisórias e cards. Hover do primário com `--accent-hover` (#19665e com texto branco 6,77:1; #7ae0c4 com texto escuro 11,27:1). Estado desabilitado com `--disabled-bg` e `--disabled-text`, sem opacidade sobre o texto. Gráficos: `--chart-used`, `--chart-free`, `--chart-protected`. A marca usa `--brand-arc-outer`, `--brand-arc-inner` e `--brand-block` (componente `BrandMark`), recoloridos no escuro. As duas cópias da paleta escura (preferência do sistema e escolha manual) são conferidas por `tokens.test.ts`.

**Forma e espaço:** raios `--radius-badge` 6, `--radius-control` 10, `--radius-inner` 12 e `--radius-card` 18; `--space-5` 20 completa a escala de 4 px. Elevação: `--shadow` para cards e `--shadow-raised` para a barra de seleção fixa; `--shadow-dialog` para diálogos.

**Motion:** transições nomeadas com `--ease-standard`; o drawer entra com `drawer-enter` em `--motion-standard`. Movimento reduzido (sistema ou manual) remove transições, translados de hover e o arco giratório do scanner. Um arco parado pareceria progresso.

**Gráficos (ADR 0003):** dentro do gráfico, só valores medidos (ocupado e disponível). Estimativas aparecem em texto ao lado, com a consequência da política. Barras de composição por risco somam só tamanhos conhecidos e sem sobreposição (`measuredSum`), têm legenda textual e nunca mostram desconhecido como zero.

**Componentes novos:** `BrandMark` (marca inline com tokens), variante `drawer` do `Modal` (mesmo `<dialog>` modal, foco e Escape), `.switch` (checkbox com `role="switch"`), `.mix-bar` com `.mix-legend` e `.project-mix`, `.outcome-summary` (contadores do relatório) e `.scan-state` (estado da análise na sidebar).
