# Checklist de implementação

Fase 0 implementada e abertura nativa verificada; Fase 1 em andamento — 31/10/2026. Os documentos normativos originais são preservados.

## Fase 0

- [x] Workspace Rust, shell Tauri 2 sem invoke/plugins, Svelte/TS, lockfiles e CI básico configurado (CI remoto não executado).
- [x] Contratos de detector/candidato/escopo/plano/risco/resultado/evento.
- [x] Corpus sintético compartilhado, testes sem HOME real.
- [x] Arquitetura, ameaças, política de risco e ADR revisáveis.
- [x] Personas, wireframes, design system, motion e backlog P0 definidos.
- [x] Protótipo onboarding → scan de demonstração → revisão → impacto → confirmação → execução simulada → relatório.
- [x] Explainability, Project Lens, Ledger e Simulator com dados sintéticos.
- [x] Claro/escuro, janela estreita, teclado, erro/vazio/cancelamento/reduced motion verificados.
- [x] cargo fmt/clippy/test dos default-members; frontend typecheck/lint/format/test/build; 7 E2E Chromium, axe e screenshots.
- [x] Build Tauri nativo e smoke visual WebKitGTK: [evidência](evidence/native/acceptance.md).
- [ ] E2E Playwright WebKit: fallback requer libicu74/libjpeg-turbo8. Não confundir com o smoke nativo.
- [x] Abre em Fedora 44 via WebKitGTK em X11/Xvfb isolado; dependências instaladas com autorização expressa posterior. Standalone/pacotes e Wayland interativo pendentes.

## Fases seguintes — não implementadas

- [ ] F1: scanner read-only de roots aprovadas, detectores com fontes oficiais atuais, eventos/cancelamento, bytes/deduplicação/limites, adversarial traversal.
- [ ] F2: executor baixo risco dirfd/TOCTOU, planos internos efêmeros, Lixeira, medição real, testes adversariais em temporários.
- [ ] F3: Git/TF/builds com revisão individual e preservação de estado.
- [ ] F4: i18n EN-US, benchmark 100/1000/10000, 60 fps em hardware representativo, empacotamento deb/rpm, Ubuntu/Fedora/Wayland/X11.

## Gates pendentes

Usabilidade com 3 desenvolvedores e 2 pessoas não técnicas: não executada; sem participantes. Segurança real de deleção não certificada. Nenhum release/publicação/PR autorizado por este incremento. Não executar testes sobre dados reais. Próxima fase só após gates aplicáveis, registrando bloqueios de ambiente sem promover pendência a aprovado.

Evidências e comandos exatos: [Fase 0](evidence/phase0.md). Sem deleção real, testes sobre dados reais, sudo ou publicação. 14 registros npm de build/dev (2 low, 12 moderate), sem vulnerabilidade de produção na consulta registrada. Rust 1.93.0 + Zig 0.17.0 isolados em .tools.
