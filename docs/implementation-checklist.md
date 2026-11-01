# Checklist de implementação

Versão inicial: Fase 0 em execução, 31/10/2026. Os documentos normativos originais são preservados.

## Fase 0

- [ ] Workspace Rust, shell Tauri 2, Svelte/TS, lockfiles e CI básico.
- [ ] Contratos de detector/candidato/escopo/plano/risco/resultado/evento.
- [ ] Corpus sintético compartilhado, testes sem HOME real.
- [x] Arquitetura, ameaças, política de risco e ADR revisáveis.
- [x] Personas, wireframes, design system, motion e backlog P0 definidos.
- [ ] Protótipo onboarding → scan de demonstração → revisão → impacto → confirmação → execução simulada → relatório.
- [ ] Explainability, Project Lens, Ledger e Simulator com dados sintéticos.
- [ ] Claro/escuro, janela estreita, teclado, erro/vazio/cancelamento/reduced motion verificados.
- [ ] cargo fmt/clippy/test; frontend typecheck/lint/test/build; E2E e screenshots.
- [ ] Abre em Linux via WebKitGTK. Bloqueio conhecido: GTK3/WebKitGTK 4.1 ausentes; não usar sudo.

## Fases seguintes — não implementadas

- [ ] F1: scanner read-only de roots aprovadas, detectores com fontes oficiais atuais, eventos/cancelamento, bytes/deduplicação/limites, adversarial traversal.
- [ ] F2: executor baixo risco dirfd/TOCTOU, planos internos efêmeros, Lixeira, medição real, testes adversariais em temporários.
- [ ] F3: Git/TF/builds com revisão individual e preservação de estado.
- [ ] F4: i18n EN-US, benchmark 100/1000/10000, 60 fps em hardware representativo, empacotamento deb/rpm, Ubuntu/Fedora/Wayland/X11.

## Gates pendentes

Usabilidade com 3 desenvolvedores e 2 pessoas não técnicas: não executada; sem participantes. Segurança real de deleção não certificada. Nenhum release/publicação/PR autorizado por este incremento. Não executar testes sobre dados reais. Próxima fase só após gates aplicáveis, registrando bloqueios de ambiente sem promover pendência a aprovado.
