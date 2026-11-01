# Design system

Direção herdada da marca: clean, silenciosa e precisa. Fonte local Inter se instalada, system-ui como fallback; sem font CDN. Grid 4px, espaçamentos 4/8/12/16/24/32, corpo 14–16px, títulos 24/30px, números tabulares. Cards 18px e controles 10px; sombras discretas. SVGs originais importados diretamente de assets/brand, sem redesenho.

CSS em apps/desktop/src/styles/tokens.css: ink #14202B, deepTeal #164F52, teal #1F7A71, mint #58D5B4, canvas #F7FAF8, surface #FFFFFF, stroke #DDE6E3, muted #64747C, warning #AD6B17, danger #B54646. Cores semânticas sobre fundos claros usam variantes escuras para AA; dark #10191D/#18262B/#E8F1EE e mint como ação com texto escuro. Preferência automática do sistema + escolha explícita. Alto contraste usa bordas/forced-colors.

Primitivos: button nativo (primary/secondary/quiet), badge de risco com ícone/texto, card categoria, lista selecionável, anel com legenda, dialog nativo, campo de busca e status anunciado. Estados hover/focus-visible/active/disabled/loading/error; sem sucesso optimista. Catálogo interno exercita loading/empty/error/partial/paused e controles.

Metas: contraste texto ≥4.5:1, UI/foco ≥3:1; alvos ≥36px (44px em viewport estreito), foco visível; leitor de tela e teclado. Auditoria automática é evidência parcial, não certificação WCAG. Listas grandes/virtualização pertencem à F1/F4; corpus limitado da F0 não demonstra escala.


F4: idioma PT-BR/EN-US acompanha valores numéricos e datas, preservando caminhos e nomes definidos pelo usuário. Histórico usa os mesmos tokens de superfície/stroke/texto/erro/focus; lista de outcomes pagina 100 arquivos e não cria linhas fictícias. Exclusões têm estados editing/saving/error/disabled e feedback após backend. Screenshots desktop/reflow estão em `evidence/f4`; controles nativos seguem o tema/idioma do sistema além do título localizado do app.


0.1.2 inclui Inter Variable 4.1 localmente, sem CDN, com créditos/licença nas configurações. Fonte oficial: [rsms/inter v4.1](https://github.com/rsms/inter/releases/tag/v4.1); [SIL OFL](https://raw.githubusercontent.com/rsms/inter/v4.1/LICENSE.txt). Arquivos em assets/fonts; WOFF2 SHA-256 693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3. O novo scanner usa as superfícies, raios, tipografia e tokens de motion existentes; revela classificação provisória sem transformar inventário protegido em ganho. Guia de recuperação mantém avisos persistentes, caminho selecionável e falha explícita de clipboard.
