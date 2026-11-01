# Evidências da Fase 0

Data local: 31/10/2026. Aplicação verificada: **ec6e118ad6722645dabe68728b6a2407c8a44768**, versão 0.0.1. As atualizações posteriores desta entrega são documentação/evidência. [Hashes de fonte e build](source-sha256.txt).

## Resultado e limite de aceite

**Protótipo implementado; aceite desktop nativo pendente.** Nenhum teste lê ou limpa dados reais do usuário. Core e CLI operam sobre JSON embutido; renderer importa a mesma fixture. `supported_action` é sempre `none`; nenhuma chamada invoke de aplicação, plugin FS, shell ou executor é registrada. Isso não certifica segurança de um futuro scanner/executor.

| Verificação local | Resultado observado |
| --- | --- |
| `cargo fmt --all --check` | Passou no workspace |
| `cargo clippy --locked --all-targets -- -D warnings` | Passou para default-members: core/clean/CLI |
| `cargo test --locked` | 5 testes passaram (3 core, 1 clean, 1 CLI); doc-tests sem casos |
| CLI `scan --demo --json` | Passou, [JSON sintético](cli-scan-demo.json) |
| CLI `clean --demo --dry-run` | Passou, [plano](cli-dry-run.json): 503316480 bytes lógicos estimados, ganho medido null |
| `npm ci --ignore-scripts` | Lockfile reinstalado; checks/build posteriores passaram |
| `npm run typecheck` | 0 erros / 0 avisos do svelte-check |
| `npm run lint`, `npm run format:check` | Passaram |
| `npm test` | 4 testes Vitest passaram: seleção conservadora, deduplicação, cobertura e outcomes |
| `npm run build` | Passou; Vite 8.3.5, JS 110,02 kB bruto / 38,91 kB gzip; CSS 27,19 kB bruto / 6,02 kB gzip |
| `npm run test:e2e -- --project=chromium` | 7 testes passaram em 9,3 s na última execução |
| axe-core 4.14 | 0 violações nos contextos auditados: overview claro/escuro, diálogo de explicação, erro e relatório parcial; não certifica WCAG |
| ui_lint `apps/desktop/src --changed` | 0 erro / 0 aviso; 8 arquivos, 0 cores fora dos tokens |
| `cargo check --locked -p inodune-desktop` | Bloqueado no build de libdbus-sys: `dbus-1.pc` não encontrado; [erro preservado](native-check.txt). GTK3/WebKitGTK 4.1 também ausentes via pkg-config |
| E2E WebKit | Tentado: browser não iniciou por libicu74/libjpeg-turbo8 ausentes no fallback Ubuntu 24.04 em Fedora 44; nenhum comportamento WebKit aprovado |
| CI remoto / aprovação humana / produção | Não executados / não obtida / não publicado |

Tooling local: Fedora 44 KDE, Node 24.18.0, npm 11.16.0, Rust 1.93.0. Rustup e Zig 0.17.0 foram instalados em `.tools/`, sem mudar PATH/configuração global. Zig atua como linker C via wrapper `.tools/cc`; checksum do tarball conferido contra [índice oficial](https://ziglang.org/download/index.json): `1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026`. GTK/WebKit/DBus não foram instalados, e nenhum sudo foi executado.

Para repetir os comandos Cargo nesta cópia, prefixar com:

```sh
export RUSTUP_HOME="$PWD/.tools/rustup"
export CARGO_HOME="$PWD/.tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$PWD/.tools/cc"
```

Ocorrências anteriores mantidas como histórico: Rust inicialmente falhou por `cc` ausente, resolvido pelo linker isolado; WebKit continuou indisponível. Um teste de UI revelou Configurações inacessível em janela estreita, corrigido. A inspeção do relatório revelou restauração de foco no skip link ao desmontar o diálogo, corrigida e coberta por assert de foco no H1. Uma execução E2E coincidiu com reinstalação de node_modules e não iniciou o Vite; foi descartada como evidência e repetida sequencialmente após a instalação, passando.

## Jornadas e inspeção visual

E2E: onboarding → scan de exemplo → revisão → detalhe de IA protegida → Escape restaura foco → impacto da Lixeira → consentimento explícito → relatório; scan pausado/cancelado; estados vazio/erro nas telas de projetos/protegidos; permanente com item alterado/falha; cancelamento entre itens com preservação de outcomes e estimativa congelada; Project Lens via teclado e auditoria de que o renderer só solicita recursos locais.

Capturas da **saída de produção**, abertas e inspecionadas:

- [Dashboard claro, 1440×1000](chromium-overview-light.png) e [escuro](chromium-overview-dark.png).
- [Janela estreita escura, 390×844](chromium-narrow-dark.png) e [Configurações estreitas](chromium-settings-narrow.png).
- [Análise pausada](chromium-scan-paused.png), [erro persistente](chromium-scan-error.png) e [relatório parcial](chromium-report-partial.png).

Observação: o dashboard coloca o potencial lógico ao lado do volume, com ocupação/livre em legenda textual. Os cards distinguem inventário protegido e potencial, sem somar worktree principal ao target. No escuro, as superfícies preservam separação por bordas e a ação usa mint. Em janela estreita, o volume e as linhas se empilham, os caminhos quebram linha e Configurações continua alcançável pela navegação horizontal. Em janelas baixas a sidebar tem rolagem própria. No relatório, falha e item alterado permanecem visíveis; o espaço real continua “Não medido”.

Defeitos encontrados e corrigidos: texto auxiliar inicialmente abaixo de 12px; inventário IA/Git apresentado como 0 B de potencial sem expor ocupação; Configurações escondida em janela estreita; foco/restauração após confirmação; bordas capturadas durante transição de tema (screenshots agora finalizam animações para determinismo). Não há confete, porcentagem de progresso por timer nem medição falsa de ganho.

Rubrica de autoavaliação do protótipo (0–3): hierarquia 3, composição 3, tipografia 2, cor 3, espaço/densidade 3, componentes/estados 3, adaptação 2, acessibilidade 2, conteúdo 3, identidade/polimento 2. Média 2,6. É avaliação de implementação e screenshots, não pesquisa com pessoas nem certificação do produto nativo.

Checks de acessibilidade efetivamente realizados: nomes/roles, diálogo nativo com Escape/restauração de foco, foco no H1 ao navegar e após confirmar, fluxo via Enter, `prefers-reduced-motion` eliminando animationName do diálogo e opção manual persistida sem reativar motion sobre o OS. Sem overflow horizontal nos contextos verificados em 390px e 320px. O preview compartilhado do T3 também abriu o build e navegou à revisão; console sem exceções e requisições apenas localhost. A aba e o servidor de inspeção foram encerrados.

## Pendências e segurança de dependências

- WebKitGTK/Tauri real, Ubuntu, Wayland/X11, distribuição deb/rpm e CI remoto não verificados.
- Não existem traversal real, testes de symlink/mount/hardlink/TOCTOU ou executor seguro. Entram nas fases seguintes, sempre em temporários isolados.
- 60 fps, atraso máximo de 100ms, virtualização e benchmarks 100/1000/10000 não medidos. Nenhum número de performance foi inventado.
- Sem participantes: validação com 3 devs + 2 usuários não técnicos pendente. Leitor de tela real, zoom 200%, alto contraste em runtime e EN-US também pendentes.
- [npm audit completo](npm-audit.json): 14 registros transitivos de tooling (2 low, 12 moderate), majoritariamente dois advisories: [deepmerge prototype poisoning](https://github.com/advisories/GHSA-gcqx-w48m-w2h4) e [minimatch DoS](https://github.com/advisories/GHSA-g3hv-75wg-m249). Não houve downgrade/override forçado; o npm não indica patch dentro dos ranges usados. Dev server fica em loopback; só glob/config do repositório são usados no tooling, sem conteúdo de roots reais.
- [Audit de produção](npm-audit-production.json): zero ocorrências na consulta. Isso não substitui revisão de supply chain; cargo-audit não executado.
- Revisão independente de IA: apontou hipótese de conclusão antecipada; leitura de `startExecution` mostrou `executing=true` síncrono antes da troca de tela, sem reproduzir a hipótese. E2E agora também afirma “Processando” e ausência de conclusão enquanto pendente. Não é aprovação humana.

O próximo incremento é a Fase 1, após resolver/documentar os gates de ambiente. Não foi iniciado scanner real nesta entrega.
