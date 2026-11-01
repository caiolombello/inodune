<p align="left"><img src="assets/brand/inodune-logo.svg" width="240" alt="Inodune" /></p>

Liberte espaço. Preserve o essencial.

Aplicativo Linux open source (MIT), com Rust + Tauri 2 + Svelte/TypeScript. **Fase 1: aplicativo desktop com scanner Rust somente leitura.** Escolha pastas específicas no diálogo nativo e revise metadados locais. Nenhum executor de limpeza foi habilitado ainda. Demonstração com dados sintéticos disponível separadamente. Fases 2–4 em desenvolvimento.

![Protótipo claro — valores fictícios](docs/evidence/chromium-overview-light.png)

## Executar o protótipo

Node **24.18.0** e npm (versão usada: 11.16.0). Sem conta, API key, CDN ou telemetria. Rede necessária para instalar dependências; o protótipo funciona localmente depois do build.

```sh
npm ci
npm run dev
# http://127.0.0.1:1420
```

Para verificar o build de produção:

```sh
npm run build
npm run preview -w @inodune/desktop
```

O onboarding é dispensável. Explore análise pausável/cancelável, revisão por item, busca/risco/ordenação, detalhes do detector, Projetos (Project Lens), Itens protegidos (Protection Ledger), simulador de impacto, confirmação e relatório parcial. Configurações permite reproduzir cenário vazio, erro de permissão e itens alterados/falhas. O catálogo mostra componentes e estados. Tema claro/escuro/automático e redução manual de movimento; a preferência do OS também prevalece.

No modo Demonstração, todos os paths começam com `/demo/` e são texto de fixture. No desktop local, o scanner acessa somente roots aprovadas, sem seguir symlinks/mounts; o volume é medido por statvfs e o progresso vem do Rust. A UI não abre essas pastas. Só preferências visuais são persistidas; seleção, caminhos e relatório duram na memória da sessão. Os únicos comandos de aplicação são aprovação por picker, listagem/esquecimento de scopes e início/progresso/controle de scan por IDs. Sem API FS/shell/rede genérica nem deleção exposta. A simulação TypeScript é descartável como executor: não deve ser conectada a operações destrutivas.

## Workspace e CLI

```text
crates/inodune-core    contratos serializáveis e política de simulação
crates/inodune-clean   dry-run puro; nenhum executor ou acesso ao FS
crates/inodune-cli     CLI explicitamente sintética, mesmo core
apps/desktop          Svelte 5 / TypeScript / Vite 8 e shell Tauri 2
assets/brand          assets canônicos fornecidos
tests/fixtures       corpus JSON compartilhado Rust/TS (sem dados pessoais)
docs                  arquitetura, ameaças, UX, checklist e evidências
```

Com Rust **1.93.0** e compilador/linker C instalados:

```sh
cargo run --locked -p inodune-cli -- scan --root /pasta/aprovada --json
# Demonstração sem filesystem:
cargo run --locked -p inodune-cli -- scan --demo --json
cargo run --locked -p inodune-cli -- clean --demo --dry-run
```

`scan --root /pasta/aprovada --json` já usa o scanner real (somente leitura); nenhuma execução sobre dados reais foi usada na verificação. A CLI exige roots explícitas, recusa comandos não reconhecidos e `--apply` com exit code 2; cobertura parcial retorna exit code 3. Ctrl+C solicita cancelamento com relatório parcial. `measured_freed_bytes: null` significa **não medido**, não zero recuperado. Nenhum resultado sintético autoriza exclusão. Tamanhos lógicos são estimativas e não garantem ganho físico. Lixeira no mesmo volume não libera espaço imediatamente.

## Verificar

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
npm run typecheck
npm run lint
npm run format:check
npm test
npm run build
npx playwright install chromium webkit
npm run test:e2e
# Só Chromium se WebKit estiver indisponível (isso não aprova o WebKit):
npm run test:e2e -- --project=chromium
```

Os comandos Cargo acima usam `default-members` (core, clean e CLI). **Não incluem o shell desktop.** `cargo check --locked -p inodune-desktop` verifica o shell separadamente. Os testes usam corpus sintético ou árvores temporárias criadas pelo teste, nunca dados ativos. F1 já cobre symlinks, bind mounts em namespace privado, hardlinks, raiz trocada e estados incertos; testes de executor/TOCTOU destrutivo entram na F2, também só em fixtures.

A suíte E2E inicia e encerra seu servidor de produção na porta 1420, que deve estar livre, e grava screenshots em `docs/evidence/`. Veja [resultados e limites da Fase 0](docs/evidence/phase0.md). CI está configurado, mas não foi executado em serviço remoto.

## Desktop Linux e provisionamento

[Tauri 2 exige bibliotecas nativas](https://v2.tauri.app/start/prerequisites/). **Provisionamento nativo já foi autorizado e executado neste Fedora**, sem conceder root ao app. Os comandos abaixo permitem reproduzi-lo em outra instalação autorizada. Instale Rust pelo [rustup oficial](https://rustup.rs/) e o toolchain fixado em `rust-toolchain.toml`.

Ubuntu/Debian (nomes de pacotes devem estar disponíveis na versão da distribuição):

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev
```

Fedora:

```sh
sudo dnf install gcc gcc-c++ make pkgconf-pkg-config webkit2gtk4.1-devel gtk3-devel \
  openssl-devel libxdo-devel libappindicator-gtk3-devel librsvg2-devel dbus-devel
```

Depois:

```sh
npm ci
npm run desktop
```

O app roda com UID comum; `sudo` não é usado no aplicativo. Abertura nativa e jornada WebKitGTK/IPC foram verificadas em Fedora/X11 virtual. Ubuntu e Wayland interativo seguem pendentes. O standalone debug já foi verificado sem Vite; os pacotes ainda estão por implementar. O bundle continua desabilitado; `.deb`/`.rpm` são alvos futuros, sem releases publicados. Não há suporte declarado a Flatpak/AppImage nesta fase.

Neste ambiente Fedora 44, Rust e Zig foram instalados **apenas em `.tools/`**, ignorado pelo Git. Para repetir os checks Rust nesta cópia, que não tem `cc`, use:

```sh
export RUSTUP_HOME="$PWD/.tools/rustup"
export CARGO_HOME="$PWD/.tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$PWD/.tools/cc"
cargo test --locked
```

O wrapper `.tools/cc` chama Zig 0.17.0. Esse caminho é uma conveniência local, não uma dependência do produto nem um substituto de GTK/WebKit/DBus. Nenhuma configuração global do shell/Git foi alterada.

## Segurança, escopo e pendências

- [Arquitetura](docs/architecture.md), [ameaças](docs/threat-model.md), [política de risco](docs/risk-policy.md) e [checklist](docs/implementation-checklist.md).
- [Jornadas](docs/ux-personas-and-journeys.md), [wireframes](docs/ux-wireframes.md), [design system](docs/design-system.md), [motion](docs/motion-spec.md) e [inovações](docs/innovation-backlog.md).
- Fase 1 implementada: scanner read-only, roots por diálogo nativo, limites/cancelamento, inventário protegido, GUI/CLI compartilhados, Project Lens/Ledger/Explainability e Simulator com metadados de fixtures reais. [Regras e fontes](docs/detectors.md), [evidências](docs/evidence/f1/acceptance.md). Git ainda é marcador protegido até F3; locais não documentados ficam protegidos.
- Fases 2–3: executor seguro, medição de espaço, revalidação TOCTOU e Git/Terraform; não implementados. Idade não comprova desuso. Estado, sessões/memória e toda incerteza permanecem protegidos.
- Pendentes: Pacotes distribuíveis, testes de usabilidade com participantes, leitor de tela real, zoom 200%, desempenho nativo/60 fps (virtualização e baseline sintético de 10.000 itens já verificados), EN-US, empacotamento e revisão humana antes de release destrutivo. Nenhum desses gates é considerado aprovado.
- Auditoria npm: sem vulnerabilidades de produção na consulta registrada; 14 registros de build/dev (2 low e 12 moderate), principalmente `deepmerge` (prototype poisoning) e `minimatch` (DoS por glob). Sem patch compatível indicado pelo npm; não se considera o tooling livre de vulnerabilidades. Consulte `docs/evidence/npm-audit.json`; não foi usado downgrade automático/forçado.

Os documentos originais `INODUNE_PLAN.md`, `INODUNE_BRAND.md` e `UX_MOTION_INNOVATION.md` seguem normativos. O PNG conceitual não é evidência da implementação. Fonte de sistema sem distribuição de Inter; ícones Lucide sob ISC, dependências com suas próprias licenças. Verificar exclusividade do nome antes de lançamento público.

## Teste do desktop real com fixtures

Somente para desenvolvimento, nunca habilitar a feature de fixtures em pacote distribuído:

```sh
cargo build --locked -p inodune-desktop --features fixture-tests
python3 scripts/native-e2e.py
```

O harness usa Xvfb próprio, XDG isolado, tauri-driver 2.1.0 e WebKitWebDriver correspondente ao sistema. As roots são criadas pelo próprio binário em temporário, sem aceitar path de teste por JS. Os binários de driver nesta cópia estão em `.tools/`; siga o [setup oficial](https://v2.tauri.app/develop/tests/webdriver/manual-setup/) para outra máquina. `scripts/native-smoke.py` é o smoke visual simples. O executável padrão não cria fixtures nem aprova pastas automaticamente.
