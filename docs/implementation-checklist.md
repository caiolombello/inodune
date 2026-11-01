# Checklist de implementação

Fases 0–2 implementadas e verificadas localmente; Fase 3 é o próximo incremento — 01/11/2026. Os documentos normativos originais são preservados.

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

## Fases seguintes

- [x] F1: scanner read-only de roots aprovadas, fontes oficiais, GUI/CLI compartilhados, contagens/cancelamento/pausa, bytes/deduplicação/limites, testes adversariais e native E2E em fixtures. Git permanece marcador protegido até F3. Ver [evidência](evidence/f1/acceptance.md).
- [x] F2: executor de folhas baixo risco por dirfd/captura privada, planos efêmeros de uso único, confirmação nativa, Lixeira/permanente, journal/histórico privado, variação observada sem atribuição, relatórios parciais e testes adversariais/CLI PTY/WebKitGTK em temporários. [Evidência e limites](evidence/f2/acceptance.md).
- [ ] F3: Git/TF/builds com revisão individual e preservação de estado.
- [ ] F4: i18n EN-US, benchmark 100/1000/10000, 60 fps em hardware representativo, empacotamento deb/rpm, Ubuntu/Fedora/Wayland/X11.

## Gates pendentes

Usabilidade com 3 desenvolvedores e 2 pessoas não técnicas: não executada; sem participantes. Segurança real de deleção não certificada. Nenhum release/publicação/PR autorizado por este incremento. Não executar testes sobre dados reais. Próxima fase só após gates aplicáveis, registrando bloqueios de ambiente sem promover pendência a aprovado.

Evidências e comandos exatos: [Fase 0](evidence/phase0.md). Na F0 não houve deleção; na F2 operações reais foram executadas exclusivamente sobre fixtures temporárias criadas pelo teste. Nenhuma limpeza/teste sobre dados reais nem publicação. Provisionamento nativo com sudo foi autorizado expressamente; o app não usa root. 14 registros npm de build/dev (2 low, 12 moderate), sem vulnerabilidade de produção na consulta registrada. Rust 1.93.0 + Zig 0.17.0 isolados em .tools.
