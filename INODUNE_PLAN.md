# Inodune — plano de produto e implementação

> **Documento mestre para o Claude Code** · v0.2 · 31/10/2026  
> **Objetivo:** desenvolver um aplicativo desktop para Linux, bonito e confiável, que encontre desperdício de espaço — especialmente em ambientes de desenvolvimento — e permita recuperá-lo com controle explícito do usuário.

## 0. Decisões de produto

- **Nome de trabalho:** Inodune (`inodune`). Combinação inspirada em *inode* (conceito de sistemas de arquivos Unix) e *dune* (leveza, espaço). Nas buscas web realizadas em 30/10/2026 não apareceu produto de software com correspondência exata; isso **não é garantia de exclusividade, registro de marca ou disponibilidade de domínio**. Confirmar em INPI, WIPO, GitHub, registries de pacotes e domínios antes do lançamento público.
- **Posicionamento:** “Liberte espaço. Preserve o essencial.” / “Make room for what matters.”
- **Público principal:** desenvolvedores, DevOps/SRE, usuários Linux avançados. **Público secundário:** usuários comuns, com interface compreensível sem saber o que é `.terraform`.
- **Princípio:** a confiança é mais importante que o número de gigabytes prometidos. **UX/UI premium, motion design e diferenciação são requisitos funcionais com testes e aceite**, não apenas polimento final.
- **Privacidade:** 100% local, sem login, sem IA remota, sem telemetria por padrão, sem rede necessária para escanear/limpar.
- **Plataforma inicial:** distribuições Linux desktop Ubuntu/Debian e Fedora, sessão Wayland e X11, arquitetura x86_64. Expansão para Arch e aarch64 após MVP.
- **Definição de MVP:** escanear, explicar, filtrar e executar limpeza segura de caches de usuário/dev, com workflows revisáveis para Git e Terraform. Os rastros de agentes de IA entram inicialmente em modo **inventário + revisão manual**, sem exclusão automática de históricos/memória.

## 1. Experiência do usuário

### Navegação

1. **Visão geral:** espaço por volume, espaço ocupado/livre, candidatos encontrados, potencial **estimado** de recuperação e botão “Revisar”.
2. **Desenvolvimento:** caches de linguagens e gerenciadores, artefatos de build, Terraform.
3. **Git Worktrees:** worktrees identificadas por repositório, último indício de atividade, branch, situação e riscos.
4. **IA & Agentes:** arquivos temporários, logs, transcripts, sessões e memória de Claude Code, Codex, OpenCode e outras ferramentas **apenas quando seus formatos forem documentados e validados**.
5. **Sistema:** XDG cache, thumbnails, caches de aplicativos. Operações administrativas separadas e não incluídas no MVP.
6. **Histórico:** relatórios locais, ações, erros, itens enviados à Lixeira e instruções para reversão quando houver.
7. **Configurações:** fontes de scan, exclusões, período mínimo, tema, acessibilidade, idioma e política de descarte.

### Fluxo obrigatório

`Escolher escopos → Escanear (sem escrita) → Exibir categorias → Revisar itens → Mostrar impacto/risco → Confirmar → Revalidar → Executar → Relatório`.

- **Nunca apagar ao iniciar o app.** Escaneamento é somente leitura.
- Cada candidato mostra **origem, tamanho, motivo, última alteração, implicação da limpeza, recriação/custo estimado, riscos, e o que será preservado**.
- Seleção padrão somente de categorias **Baixo risco**, e ainda assim executar só após consentimento.
- `Revisar` e `Protegido` sempre desmarcados. `Protegido` pode ser somente inventário, sem botão de remover.
- UI distingue: **candidatos encontrados**, **potencial recuperável**, **realmente recuperado**. O potencial não pode ser contabilizado como ganho.
- Botão de “desfazer” apenas para operações realmente reversíveis; itens na Lixeira não liberam espaço imediatamente no mesmo filesystem.
- Uma “limpeza rápida” é apenas seleção sugerida + revisão + confirmação, não um atalho para exclusão silenciosa.
- Antes de apagar, apresentar novo resumo se algo mudou entre o scan e a execução.

### Estados e detalhes de UX

- Empty state instrutivo, estados de scan pausado/cancelado, erros por item, partial success, múltiplos discos, áreas inacessíveis, caminhos em mounts externos.
- Filtro por ferramenta, categoria, risco, idade, tamanho e tipo; busca por caminho; ordenação; seleção em lote com contagem e soma **sem double count**.
- Destaques “Pode ser recriado”, “Exige reinicialização”, “Sessões recuperáveis”, “Estado importante”, “Em uso”.
- Compatível com teclado, leitor de tela, zoom, `prefers-reduced-motion`, alto contraste, internacionalização PT-BR e EN-US.
- Preferência visual: interface clara predominante com dark mode completo, sem animação excessiva, estilo editorial, cartões suaves e excelente hierarquia de informação.

### UX/UI, animações e inovação como entregas obrigatórias

O documento [`UX_MOTION_INNOVATION.md`](UX_MOTION_INNOVATION.md) é parte **normativa** deste plano. Claude Code precisa implementá-lo junto com as regras de segurança. Ele define onboarding, telas, estados, tokens de animação, inovação, testes com usuários e métricas.

**Exigências principais:**

- Produzir protótipo navegável com estados reais de scan, seleção, risco, erro, cancelamento, execução e relatório **antes de habilitar a limpeza destrutiva**.
- Aplicar movimento intencional (feedback, orientação espacial, conclusão), com transições curtas, preferência de redução de movimento, desempenho medido e nenhuma animação que oculte operações pendentes.
- Não deixar beleza para a Fase 4: o design system, hierarquia, componentes e motion tokens entram já na Fase 0; na Fase 1 o produto precisa parecer utilizável e bem-acabado.
- Inovações do MVP: **Cleanup Explainability**, **Space Budget Simulator**, **Project Lens** e **Protection Ledger**. Todas com estado vazio, carregamento, erro e explicações acessíveis.
- Registrar decisões de UX em arquivos versionados, screenshots por fase e critérios de aceite. Avaliar protótipo com usuários quando houver acesso a participantes; caso contrário, manter esta validação como pendente, nunca afirmar que ocorreu.
- **Não gamificar deleções:** sem confete ou feedback enganoso de “vitória” antes de medir o ganho real, sem animação que pressione confirmação irreversível.

## 2. Stack e arquitetura

**Recomendação:** Rust como única autoridade sobre filesystem + **Tauri 2** (desktop shell) + **Svelte/TypeScript** (frontend) + CSS tokens. Essa combinação permite UX consistente em GNOME/KDE sem misturar regras de deleção com a interface. O WebKitGTK é uma dependência real no Linux e precisa constar na documentação/distribuição.

Estrutura de projeto proposta:

```text
inodune/
  Cargo.toml                 # workspace
  crates/
    inodune-core/            # inventário, detectores, risco, plano, métricas, políticas
    inodune-clean/           # revalidação e execução, sem chamadas da UI
    inodune-cli/             # scan / explain / plan / clean --dry-run
  apps/desktop/
    src-tauri/               # Tauri v2: comandos estritamente tipados e capacidades mínimas
    src/                     # Svelte + TypeScript, telas, tokens, componentes
  assets/brand/              # SVG canônico e exports PNG
  docs/                     # threat model, decisão de arquitetura, regras de detectores
  tests/fixtures/            # árvores falsas, repos Git, TF simulados; sem tocar home real
  packaging/                 # deb/rpm/AppImage e scripts de release
  .github/workflows/         # check, lint, test, build, release
```

**Fronteiras de confiança:** frontend é não confiável. Nenhum `invoke` de Tauri recebe um caminho e realiza exclusão arbitrária. A UI só pode pedir `scan`, `get_candidates`, `prepare_plan(plan_items)`, `execute_plan(plan_id, confirmation)` e `get_history`; o backend reconstrói validações e aplica política. Configurar *capabilities* explícitas no Tauri; não habilitar shell, FS arbitrário ou execução irrestrita a partir de JS.

**Core reutilizável:** a CLI usa os mesmos detectores e executor da GUI. Não criar duas implementações. API interna assíncrona com eventos de progresso, cancelamento, limites de concorrência, logs estruturados locais sem conteúdos de arquivos.

**Persistência:** diretórios XDG `config`, `state` e `cache` do Inodune, com permissão apenas do usuário; banco SQLite local opcional para resultados e histórico, versionado. Não salvar conteúdo de código, `.env`, transcripts ou tokens, nem enviar dados. Exibir caminhos somente na própria UI.

**Distribuição:** priorizar `.deb` e `.rpm`, avaliar AppImage. Flatpak exige solução específica para acesso ao filesystem do host (sandbox) e não deve ser anunciado como completo sem funcionar de fato. Sem privilégios root no MVP. Operações do sistema que exijam `polkit` serão plug-in separado e futuro, nunca `sudo` automático.

## 3. Motor de descoberta

### Modelo de detector

Um detector declara: `id`, versão, escopos suportados, padrões/regras **positivas e negativas**, método de validação, política de risco, prova de recriação, pré-requisitos, classificação de dados sensíveis, estratégia de limpeza, documentação e conjunto de testes.

**Contrato de candidato:**

```json
{
  "id": "opaque-stable-id",
  "detector_id": "rust.target",
  "scope_id": "approved-projects",
  "display_path": ".../project/target",
  "file_type": "directory",
  "st_dev": 0,
  "st_ino": 0,
  "mtime_unix": 0,
  "logical_bytes": 0,
  "allocated_bytes_estimate": 0,
  "risk": "low|review|protected",
  "selected_by_default": false,
  "reason": "build output reproducible",
  "rebuild_hint": "cargo build",
  "evidence": ["cargo-manifest-present"],
  "active_or_changed": false,
  "supported_action": "trash|permanent|none"
}
```

`0` no exemplo é apenas placeholder: no código, usar tipos opcionais e IDs estáveis. O manifesto final deve registrar dev+inode/metadata relevantes, versão de detector e hash do plano, mas **nunca confiar no manifesto antigo** como autorização suficiente para exclusão.

### Escopo inicial

- Home atual: XDG cache e diretórios de ferramentas detectadas.
- Pastas de projetos: o usuário escolhe 1+ roots no primeiro uso; não varrer indiscriminadamente `/`, `~` inteiro de modo recursivo ou unidades externas.
- Ignorar por padrão: roots e mounts do sistema, `.git` internos, repositórios protegidos, arquivos sensíveis, symlinks, sockets e pseudo-filesystems, caminhos sem permissão, espaços de outros usuários, diretórios que sejam mounts distintos (salvo inclusão explícita).
- Permitir exclusões por caminho e por detector. Nunca cruzar symlink ao atravessar a árvore. Hardlinks, reflinks, sparse files, CoW e snapshots podem tornar ganho físico incerto; estimativa separada do espaço efetivamente liberado medido pelo filesystem.
- Candidatos de detectores sobrepostos devem ser deduplicados. Caches de build dentro de worktree exigem decomposição para evitar dupla contagem.

## 4. Catálogo de limpeza e política

| Detector | O que encontra | Padrão | Como executar |
| --- | --- | --- | --- |
| `xdg.cache` | Caches XDG do próprio usuário, classificados por aplicativo | **Revisar** inicialmente; subset validado pode virar baixo risco | Remover entradas conhecidas, não toda `~/.cache` cegamente |
| `thumbnails` | Cache de miniaturas XDG | Baixo risco | Recriação automática; excluir apenas caminho validado |
| `python.pycache` | `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache` em projetos aprovados | Baixo risco após verificação | Não entrar em `.venv` sem detector específico |
| `js.build` | `.next/cache`, caches de Vite e ferramentas com estrutura validada | Baixo risco / Revisar | Não excluir `.next` inteiro sem distinguir artefatos |
| `rust.target` | `target/` em projetos Cargo | Revisar (alto custo de rebuild) | Confirmar manifest e não-execução de builds |
| `terraform.providers` | `.terraform/providers` em root TF detectado | Revisar | Exigir confirmação de reinit e validação de links |
| `terraform.modules` | `.terraform/modules` em root TF detectado | Revisar | Nunca apagar outras entradas em `.terraform` implicitamente |
| `git.worktree` | Worktrees registradas por Git | Revisar individualmente | Usar comandos nativos Git e *jamais* `--force` no MVP |
| `ai.temp` | Diretórios temporários explicitamente documentados por ferramenta/versão | Revisar | Somente caches inequivocamente regeneráveis |
| `ai.history` | Sessions, JSONL, transcripts e memórias de agentes | **Protegido / inventário** | Nenhuma exclusão no MVP |
| `package.managers` | Cache de `pip`/`uv`/npm/pnpm/Cargo após validação | Revisar inicialmente | Respeitar política do gerenciador; estimar custo de download |

**Regras do Terraform:** `.terraform` **não é sempre descartável**. Ela guarda providers/módulos, mas também metadados de backend e workspace. O Inodune deve classificar subpastas, preservar `.terraform/terraform.tfstate`, `.terraform/environment`, arquivos `.tfstate*`, `.terraform.lock.hcl`, `.tfvars*`, `.terraformrc`, credenciais e planos; jamais remover `.terraform` inteira como operação genérica. Verificar diretório de trabalho, links, backend e uso ativo. Limpeza permitida somente com revisão/consentimento, informando que `terraform init` poderá ser necessário. Cache global `TF_PLUGIN_CACHE_DIR`/`plugin_cache_dir` apenas em fase posterior, com política de versões e prova de não uso.

**Regras do Git:** separar `git worktree prune` (remove **metadados de worktrees ausentes**) de `git worktree remove` (remove uma **worktree real**). Usar `git worktree list --porcelain -z`; identificar principal, locked, prunable, detached, dirty/untracked, branch, upstream e commits não publicados. Idade **nunca é prova de desuso**. Recusar corrente/principal/locked/dirty/untracked/não rastreada/remota desconhecida/commits exclusivos ou qualquer estado incerto. A UI pode listar esses casos como protegidos. Antes da operação, revalidar repo e worktree, identificar processos em uso quando possível e executar sem `--force`; nenhuma limpeza automática, mesmo após vários dias. Em fase posterior, pode haver modo avançado conscientemente arriscado, nunca default.

**Regras de IA:** `~/.claude/projects/...` pode conter sessões retomáveis e memória persistente. Não tratar JSONL, arquivos de configuração, `CLAUDE.md`, `AGENTS.md`, skills, MCP, auth ou traces como “lixo” por idade/extensão. Codex/OpenCode e outros têm formatos variáveis: somente adicionar detector com documentação atual, fixtures e lista explícita de exclusões. Metadados de projeto e prompts são potencialmente sensíveis. Nunca fazer upload ou enviar para modelo para “classificar”. Na fase inicial mostrar espaço e fornecer opção “Abrir pasta”, com contexto sobre o que seria perdido.

**Fora do MVP:** Docker volumes/images/containers, Kubernetes contexts, bases locais, snaps/Flatpak runtimes, journals de sistema, APT/DNF e Browser cookies/sessions. Futuramente, usar ferramentas oficiais com confirmação e isolamento de permissões; volumes e dados persistentes ficam protegidos.

## 5. Matriz de risco e descarte

| Classe | Exemplo | Seleção default | Ação permitida |
| --- | --- | --- | --- |
| **Baixo** | `__pycache__`, miniaturas conhecidas | Sim, **após scan**, mas nunca executa sozinha | Lixeira ou exclusão permanente explícita com confirmação |
| **Revisar** | `target/`, `.terraform/providers`, worktree limpa | Não | Revisão individual + revalidação + confirmação |
| **Protegido** | tfstate, `.git`, memória/sessões de IA, dados do usuário, credenciais | Não | Nenhuma no MVP |

- **Lixeira** = reversível em geral, mas **não libera espaço físico** se continuar no mesmo volume. UX precisa comunicar isso antes da ação e no relatório.
- **Exclusão permanente** = espaço recuperável, mas irreversível. Exigir revisão e confirmação clara; risco médio requer consentimento individual ou por grupo pequeno. Não oferecer “esvaziar Lixeira” automaticamente.
- Não mostrar “gigabytes recuperados” até medir novamente `statvfs`/filesystem. Resultados podem diferir por CoW/snapshots/hardlinks, arquivos abertos e realocação concorrente.
- Em caso de dúvida, **falhar fechado**: classificar como Protegido ou ignorar o item.

## 6. Segurança do executor (obrigatório antes da limpeza)

1. **Sem shell genérico:** nunca montar `rm -rf`, `bash -c`, `sh -c`, `find -delete` a partir dos dados. Usar Rust FS API segura ou comando Git com executável e argumentos validados, sem interpolação/shell; binário Git confiável.
2. **Conter escopos:** cada plano só referencia um root aprovado. Canonicalização isolada não resolve symlink race; considerar traversal ancorado em `dirfd`, `openat`/`openat2`, `O_NOFOLLOW`, `fstatat`, `unlinkat` e validação contínua em Linux; evitar seguir links ou ultrapassar mounts.
3. **TOCTOU:** imediatamente antes de cada exclusão, comparar tipo, `st_dev`, `st_ino`, metadata relevantes, ancestral e política atual. Mudou? Pular e reportar, nunca adivinhar.
4. **Sem elevação automática:** rodar com UID do usuário; nunca alterar arquivos root/terceiros. Diretórios com permissões insuficientes = item ignorado.
5. **Operações que preservam dados:** worktrees via Git nativo; excluir apenas paths determinísticos verificados em demais detectores. Nunca modificar `.git`, arquivos de estado ou config protegidos.
6. **Cancelamento:** parar antes da próxima operação, apresentar parcial, nunca fingir rollback automático de exclusão permanente.
7. **Resiliência:** falhas parciais e diretórios alterados são esperados. Sem loop infinito, sem stack overflow ao percorrer árvore, sem memória descontrolada em diretórios enormes.
8. **Logs auditáveis e privados:** registrar o tipo de decisão e outcome; ocultar segredos e evitar conteúdo de arquivos. Histórico configurável, exportação manual local.
9. **Threat model:** symlink trocado, bind mount, path traversal, hardlink, arquivos abertos, interrupção de energia, mismatch entre UI/backend, manifest adulterado, race Git, backend Terraform ativo, dupla contagem e ferramenta de terceiros maliciosa em `PATH`.
10. **UX de consentimento:** nenhum caminho enviado pela UI deve se transformar em alvo direto de exclusão; IDs opacos e plano assinado/validado internamente no processo, com expiração curta e confirmação associada à seleção atual.

## 7. Design system (arquivos em `assets/brand/`)

Conceito: **um anel de armazenamento que ganha uma abertura**, com pequeno quadrado desacoplado. Sem vassouras, foguetes, mascotes ou metáforas infantis. Tipografia editorial simples, espaçamento generoso, foco no estado real da limpeza.

| Token | Valor |
| --- | --- |
| `ink` | `#14202B` |
| `deepTeal` | `#164F52` |
| `teal` | `#1F7A71` |
| `mint` | `#58D5B4` |
| `canvas` | `#F7FAF8` |
| `surface` | `#FFFFFF` |
| `stroke` | `#DDE6E3` |
| `muted` | `#64747C` |
| `warning` | `#AD6B17` |
| `danger` | `#B54646` |

- Fonte: **Inter** (SIL OFL, incluir fonte de modo compatível com licença) ou fallback `system-ui`; títulos `600–700`, corpo `400–500`; números tabulares.
- Grid: múltiplos de 4px; cards raio 16–20px; controles 10–12px; margem principal 24–32px; bordas suaves; sombras sutis.
- Dark mode: `#10191D` fundo, `#18262B` superfície, `#E8F1EE` texto, verde/mint ajustados para contraste WCAG AA.
- Cores de risco sempre acompanhadas por texto/ícone, nunca cor isolada.
- Ver `INODUNE_BRAND.md`, `UX_MOTION_INNOVATION.md` e `inodune-ui-concept.png` para direção, **não copiar dados fictícios nem supor que mockup representa app funcional**.

## 8. Plano de execução para Claude Code

### Fase 0 — Base e ameaças (primeiro PR)

- [ ] Configurar monorepo Rust/Tauri/Svelte, lint, formatter, lockfiles, dependências auditáveis, CI básico.
- [ ] Criar `docs/architecture.md`, `docs/threat-model.md`, `docs/risk-policy.md` e ADR de stack.
- [ ] Definir schemas de detectores, candidatos, escopos, planos, risco, resultados e eventos.
- [ ] Criar fixtures isoladas sob `tests/fixtures`; nenhum teste toca `~` real.
- [ ] Integrar design tokens, assets SVG e layout vazio com acessibilidade inicial.
- [ ] Criar protótipo navegável de dashboard→revisão→impacto→confirmação→relatório com fixtures fictícias e wireframes adaptativos. Implementar tokens de motion e suporte a reduced motion desde o início.
- [ ] Criar `docs/ux-personas-and-journeys.md`, `docs/ux-wireframes.md`, `docs/design-system.md`, `docs/motion-spec.md` e `docs/innovation-backlog.md` seguindo `UX_MOTION_INNOVATION.md`.
- **Aceite:** compila/abre em Linux alvo, `cargo test` e lint passam; documentação de segurança revisável; nenhuma API de deleção exposta ao frontend; protótipo navegável e visualmente inspecionado em claro/escuro e janela estreita, sem animações obrigatórias.

### Fase 1 — Scan somente leitura + UI útil (segundo PR)

- [ ] Implementar descoberta de roots explicitamente aprovados, cancelamento, progresso, limites e traversals sem seguir symlinks.
- [ ] Detectores `thumbnails`, `python.pycache`, `rust.target`, `terraform.providers/modules` em **inventário**, `git.worktree` em **inventário** e AI sessions em **inventário protegido**.
- [ ] API unificada GUI/CLI, cálculo de bytes lógico/alocado, deduplicação e indicação de estimativa.
- [ ] Tela geral, categorias, lista/preview, filtros, proteção, detalhes de “por que isto apareceu”.
- [ ] Implementar **Project Lens**, **Protection Ledger** e **Cleanup Explainability** com dados reais do scan; Space Budget Simulator read-only com deduplicação e riscos.
- [ ] Motion dos estados de scan, cards, filtros e detalhes, com performance real de listas virtuais e atualização de progresso sem bloquear interface.
- **Aceite:** 0 writes dentro das roots escaneadas; nenhum `tfstate` ou sessão de IA elegível para limpeza; leitura de uma árvore de testes hostil é segura e cancelável; jornada end-to-end de scan→revisão funciona com teclado, informação de risco compreensível e motion reduzido.

### Fase 2 — Executor de baixo risco (terceiro PR)

- [ ] Planos efêmeros, ID opaco, dry-run **padrão**, revalidação de fs antes de qualquer mutation.
- [ ] Implementar Lixeira e/ou exclusão permanente controlada para **apenas** thumbnails e caches Python reconhecidos (escopo aprovado).
- [ ] Diálogos de confirmação, relatório parcial, cálculo real de espaço livre antes/depois e histórico local.
- [ ] Conectar a simulação de impacto e o relatório real ao backend; estados de execução animados só depois da confirmação de cada operação e com erros visíveis.
- [ ] Testes com symlinks trocados, race, hardlinks, mount, arquivos abertos e rollback inexistente.
- **Aceite:** nenhum path fora do root autorizado é tocado, incluindo cenário adversarial; cancelamento funciona; UI nunca chama operação destrutiva sem confirmação; ninguém consegue confundir Lixeira com espaço efetivamente liberado por causa de gráfico/motion.

### Fase 3 — Worktrees, Terraform e caches de build (quarto PR)

- [ ] Git read-only robusto: `--porcelain -z`, status completo, lock, root, upstream e commits não publicados. Exibir por que bloqueou.
- [ ] Remoção opcional de worktree **somente** com `git worktree remove` não forçado, checklist de segurança e confirmação por item; prune de metadados órfãos em opção distinta.
- [ ] Terraform: remover apenas subdiretórios cache verificados com consentimento, explicar `terraform init`; nunca apagar metadados/estado/locks.
- [ ] Expandir `rust.target`, `.next/cache` e caches de gerenciadores com testes por ferramenta e opção de exclusões.
- [ ] Integrar drill-down por projeto no Project Lens, indicação de custo de rebuild e motivos de bloqueio em Git/Terraform sem novos fluxos inseguros.
- **Aceite:** casos clean/dirty/locked/detached/unpushed/no upstream/principal/alterado concorrentemente; instalações de Terraform simuladas com backend preservado.

### Fase 4 — Polimento e release (quinto PR)

- [ ] Tema claro/escuro, PT-BR/EN-US, teclado/leitor de tela, empty/error states, notificações discretas.
- [ ] Revisar consistência dos microestados (hover/focus/pressed/loading/disabled/error), motion normal/reduzido, contraste, ícones, microcopy e layouts adaptativos. Medir 60 fps alvo nas animações em hardware representativo.
- [ ] Testes de usabilidade com perfis técnico e não técnico, ou documentação explícita de validação pendente; benchmarks UI para 100, 1.000 e 10.000 itens de fixture.
- [ ] Benchmarks em home de teste, caching incremental, proteção contra diretórios enormes, logs privativos, crash reporting **local**.
- [ ] Empacotar `.deb` e `.rpm`, instruções de dependências, CI de release assinado quando infra estiver definida, SBOM e publicação de checksums.
- [ ] Testar em Ubuntu, Fedora e idealmente KDE, Wayland e X11; registrar diferenças.
- **Aceite:** instalação limpa, smoke tests, sem rede obrigatória, sem root, sem telemetria; UI reproduz direção visual, cumpre os critérios de `UX_MOTION_INNOVATION.md`, permissões e ações verificadas.

### Pós-MVP

- Catálogo de plugins/detectores assinados ou versionados **declarativos** (sem execução arbitrária); análise avançada de caches de agentes com integração de versões e consentimento individual; APT/DNF e container caches via integrações oficiais; exclusões inteligentes; modo agendado apenas depois de modelo seguro e auditável; visualização treemap para diretórios grandes; múltiplos volumes e Btrfs/ZFS awareness.
- **Não incluir IA generativa por marketing**. Classificações previsíveis, reproduzíveis e testadas têm prioridade.

## 9. Plano de testes mínimo

| Cenário | Resultado obrigatório |
| --- | --- |
| Diretório cache é substituído por symlink durante scan→clean | Item rejeitado, alvo externo intocado |
| Symlink dentro de cache aponta para `~/Documents` | Não seguir e não apagar alvo |
| Worktree com mudanças e arquivos untracked | Protegida |
| Worktree com commits não publicados/no upstream | Protegida ou exige revisão sem deleção MVP |
| Worktree principal, locked, ativa e detached | Principais/locked/ativas protegidas; detached tem checagem de commits exclusivos e revisão, nunca heurística etária |
| `git worktree prune` em repo com worktree órfã | Só metadata, nunca confundir com liberar conteúdo da worktree |
| `.terraform` com backend e `terraform.tfstate` | Caches passíveis de revisão; estado e backend intactos |
| `~/.claude/projects/.../memory` e JSONL | Inventário protegido, não apagado |
| Hardlink, sparse file e reflink | Sem dupla contagem afirmativa; ganho real medido após |
| Mount externo não aprovado | Não atravessar |
| Permissões negadas e processo de scan interrompido | Erro parcial, app continua responsivo |
| Lixeira no mesmo filesystem | Relatório não anuncia como espaço livre |
| UI envia caminho adulterado ou ID expirado | Backend rejeita |
| Limpeza interrompida entre itens | Relatório preciso dos itens processados, sem falso rollback |

Métricas de qualidade: **zero perda de dados em corpus adversarial**, nenhum acesso fora do escopo aprovado, nenhum delete antes de confirmação; tempo de scan e memória medidos por volume de fixture; lint/tests CI sem flakiness; revisão humana obrigatória antes de release de limpador destrutivo.

## 10. Critérios de pronto para V1

- App abre e funciona nas distros-alvo, GUI e CLI compartilham core.
- Usuário entende *por que*, *quanto*, *o que perde* e *como recria* cada candidato.
- Os fluxos prioritários de UX, animações com reduced motion e inovações P0 estão implementados e testados; não resta apenas um protótipo estático ou funcionalidade sem estados de erro.
- Home/código/sessões/estado não são tocados por padrões genéricos.
- Toda deleção requer plano revisado, revalidação e confirmação; relatório honesto de resultado.
- Instalação e remoção desinstalam apenas o app e os próprios arquivos; não alteram caches do sistema.
- Threat model e testes adversariais passam; binários e release verificáveis.

## 11. Referências oficiais que devem ser reconsultadas durante a implementação

1. Git worktrees — https://git-scm.com/docs/git-worktree
2. Terraform working directory — https://developer.hashicorp.com/terraform/cli/init
3. Terraform cache global — https://developer.hashicorp.com/terraform/cli/config/config-file
4. Tauri 2 Linux prerequisites — https://v2.tauri.app/start/prerequisites/
5. Tauri 2 permissions — https://v2.tauri.app/security/permissions/
6. Tauri 2 filesystem — https://v2.tauri.app/plugin/file-system/
7. Flatpak sandbox permissions — https://docs.flatpak.org/en/latest/sandbox-permissions.html
8. Claude Code sessions/memory — https://support.claude.com/pt/articles/14554922-perguntas-frequentes-do-claude-code e https://support.claude.com/pt/articles/14554000-dicas-de-usuario-avancado-do-claude-code
9. GNOME HIG — https://developer.gnome.org/hig/
10. WCAG 2.2 — https://www.w3.org/TR/WCAG22/
11. W3C reduced motion — https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions

**Importante:** padrões de diretórios, dependências e APIs mudam. Revalidar documentação oficial e versões antes de implementar cada detector. No README distinguir claramente suportado / experimental / inventário protegido.

## Extensão autorizada: inventário de programas e jogos

`inodune-apps` é uma exceção somente leitura ao escopo original do MVP e à allowlist de processos. A gramática, limites e comandos RPM/DNF, APT/dpkg, Flatpak e Snap estão em [docs/apps-readonly.md](docs/apps-readonly.md). Não autoriza remoção, escalada, shell, integração com candidatos de limpeza ou mudança das proteções do scanner existente. Steam usa exclusivamente leitura de metadados.
