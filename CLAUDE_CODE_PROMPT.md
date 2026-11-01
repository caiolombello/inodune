# Prompt de execução — Claude Code

Copie o texto abaixo no Claude Code **a partir da pasta do projeto**. O arquivo `INODUNE_PLAN.md` deve estar na mesma pasta. A ordem de trabalho é deliberada: primeiro segurança e scan, depois limpeza real.

---

Quero que você desenvolva o **Inodune**, um aplicativo Linux premium e open source de limpeza de disco, direcionado a desenvolvedores e usuários comuns. O produto deve ter UX/UI clean e moderna, com segurança e privacidade como restrições de arquitetura, não extras opcionais.

**Leia integralmente `INODUNE_PLAN.md`, `INODUNE_BRAND.md` e `UX_MOTION_INNOVATION.md` antes de alterar código.** O último é um requisito de implementação, não uma lista opcional de ideias. Use `assets/brand/inodune-logo.svg`, `assets/brand/inodune-icon.svg` e `inodune-ui-concept.png` como direção visual (este PNG é conceitual, com dados fictícios). Trate os documentos como requisitos-base; se alguma instrução for insegura, incompatível com documentação atual ou tecnicamente inviável, explique e proponha alternativa antes de seguir.

## Seu objetivo

1. Construir monorepo Rust + Tauri 2 + Svelte/TypeScript com core reutilizável por CLI e GUI.
2. Implementar detecção **somente leitura**, classificação explicável, revisão item a item, e executor de limpeza resistente a symlink/TOCTOU, com testes adversariais.
3. Priorizar inicialmente XDG thumbnails, caches Python, `target/` Rust, `.terraform/providers`, `.terraform/modules`, worktrees Git e inventário **protegido** de sessões/artefatos de IA.
4. Ter aparência de **produto final premium desde a Fase 1**: layout fluido, tipografia Inter/system, tokens do guia de marca, tema claro/escuro, motion design consistente, acessibilidade, estados reais de carregamento, erro, cancelamento e execução.
5. Empacotar para Linux (Ubuntu/Debian e Fedora primeiro), sem root, sem conta e sem telemetria.
6. Construir um **protótipo navegável antes do executor destrutivo**, com onboarding progressivo, scanner claro, revisão, simulação de impacto e relatório.
7. Implementar as inovações **Cleanup Explainability**, **Space Budget Simulator**, **Project Lens** e **Protection Ledger** no MVP, conforme `UX_MOTION_INNOVATION.md`.

## Regras inegociáveis

- Não executar nenhuma deleção real em minha máquina enquanto estiver desenvolvendo/testando. Use **somente fixtures criadas em diretórios temporários e testes isolados**. Não executar `git worktree prune/remove`, `terraform init`, remoção de caches, ou limpeza de diretórios existentes do usuário para “testar”.
- Não implementar limpeza com `rm -rf`, `sh -c`, `bash -c` ou concatenando shell commands. Não delegar autoridade de exclusão à interface.
- Não inferir que uma worktree antiga está sem uso. Git dirty, locked, main, não publicada e qualquer estado incerto devem estar protegidos.
- Não apagar `.terraform` integralmente, `.tfstate`, locks, credenciais, scripts, arquivos pessoais, nem sessões/memória de agentes.
- Não tratar um item enviado à Lixeira como espaço efetivamente recuperado até o filesystem refletir isso. Relatar estimativas e resultado real distintamente.
- Não adicionar plugins que executam código arbitrário; não embutir API keys nem fazer upload de arquivos inspecionados.
- Não pedir permissões administrativas no MVP. Não ignorar conflitos de permissão.
- Toda API, path e regra por ferramenta deve ser verificada em documentação oficial vigente. Um padrão não confirmado vira inventário protegido, não candidato de exclusão.
- Nunca exibir sucesso falso: operações parciais, arquivos alterados e itens pulados precisam aparecer.
- Nenhuma animação deve indicar sucesso antes de confirmação do backend; números e gráficos nunca devem inventar progresso ou espaço liberado. Respeitar `prefers-reduced-motion` e opção manual equivalente.
- Não entregar UI genérica sem estados de interação, screenshots/testes visuais ou navegação acessível. Não postergar toda a qualidade visual para a última fase.

## Método de trabalho

Implemente **fase por fase**, na ordem definida em `INODUNE_PLAN.md` (Fases 0 a 4). Faça primeiro a **Fase 0** e a Fase 1 antes de criar um executor destrutivo. Trabalhe em incrementos testáveis e commits pequenos, mantendo a aplicação executável ao final de cada fase. Não pule testes para acelerar entrega.

Antes de começar, entregue no repositório:

- `docs/architecture.md` com módulos, boundaries e decisão de stack.
- `docs/threat-model.md` com ameaças reais de deleção de arquivos.
- `docs/implementation-checklist.md` refletindo os critérios de aceite, status e riscos pendentes.
- `docs/ux-personas-and-journeys.md`, `docs/ux-wireframes.md`, `docs/design-system.md`, `docs/motion-spec.md` e `docs/innovation-backlog.md`, com protótipo/estados e metas verificáveis.

Para cada fase:

1. Apresente escopo e principais riscos.
2. Implemente com APIs tipadas, componentes reutilizáveis e testes.
3. Execute `cargo fmt --check`, `cargo clippy`, `cargo test`, typecheck/lint/build do frontend e smoke tests disponíveis; se o ambiente não tiver dependências, documente exatamente como provisioná-las e não invente resultados.
4. Verifique visualmente a UI local quando for possível, inclusive responsividade, tema escuro, estados vazios/erro, navegação por teclado, motion normal/reduzido e transições. Registre screenshot ou evidência; se não puder verificar, declare pendente.
5. Atualize checklist/README com funcionalidades prontas, incompletas e decisões.
6. Só depois avance à próxima fase, sem precisar pedir autorização para tarefas puramente de desenvolvimento dentro do repositório. **Peça confirmação antes de executar testes sobre dados reais, publicar releases, usar `sudo` ou realizar qualquer ação destrutiva fora de fixtures.**

## O que quero receber

- Código-fonte organizado e documentado, build reproduzível e comandos claros de instalação/desenvolvimento.
- UI com cuidado de produto, não apenas uma lista de diretórios com botões.
- Microinterações elegantes e úteis (tokens 80–320 ms), feedback consistente, dashboard que informa de verdade e listas escaláveis. Nada de animação decorativa que atrapalhe o trabalho.
- Diferenciais visíveis: simulação de ganho antes da limpeza, explicabilidade por item, agrupamento por projeto e painel de itens protegidos.
- CLI adequada para automação, com `scan --json` e `clean --dry-run` seguros por padrão; `--apply` separado, com confirmação.
- Testes cobrindo Git worktrees, Terraform, artefatos de IA protegidos, mounts, symlinks, hardlinks, corrida entre scan e limpeza, estados incertos e erros parciais.
- Documentação honesta de limitações e riscos.

Comece agora com uma leitura dos quatro documentos (incluindo este prompt), uma lista das decisões/ambiguidades realmente bloqueantes (se houver) e a implementação da **Fase 0 com protótipo navegável, design system e motion tokens**. Não escreva um segundo planejamento genérico: siga o plano existente e transforme-o em código incremental, com resultados verificáveis.
