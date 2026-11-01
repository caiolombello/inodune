# Programas & Jogos — inventário v0.1.3

Este módulo é somente leitura e independente de `inodune-core::Candidate` e do executor. Não há seleção, plano de limpeza, desinstalação, score de desuso ou persistência automática do inventário.

## Exceção restrita de processos

A exceção à política de processos externos vale exclusivamente para `inodune-apps`, com gramática fechada em `src/command.rs`: `/usr/bin/dnf --cacheonly --no-plugins --config=/dev/null --setopt=reposdir=/nonexistent repoquery --userinstalled --qf=…`, `/usr/bin/rpm -q --qf … -- ID…`, `/usr/bin/flatpak list --app --user|--system --columns=…`, `/usr/bin/apt-mark showmanual`, `/usr/bin/dpkg-query -W -f=…` e `/usr/bin/snap list --all`. Nenhuma opção de escrita, shell, sudo, pkexec ou binário obtido por PATH é permitido. O backend não recebe parâmetros da UI. IDs RPM são validados antes do spawn; nenhum ID retornado por outras fontes vira comando.

Cada subprocesso usa argumentos separados, stdin nulo, ambiente limpo com `LC_ALL=C`, PATH fixo e HOME inerte; DNF recebe um diretório temporário privado para HOME, logdir e cachedir (removido ao concluir), porque DNF5 inicializa logs mesmo em consultas. Durante esta tarefa TMPDIR aponta para a própria worktree. Nenhum estado ativo do usuário é usado para escrita; Flatpak recebe HOME e XDG_DATA_HOME para localizar a instalação de usuário. O limite agregado é 8 MiB e o prazo por consulta é 20 segundos. Em erro/timeout somente o filho iniciado pela consulta é encerrado e recolhido. As ferramentas oficiais podem fazer bookkeeping interno; não há transação de instalação/remoção nem refresh de repositórios. DNF usa cache apenas.

## Fontes e semântica

`PackageSource` separa disponibilidade de consulta. Ausência não impede as demais fontes; falhas e cobertura parcial aparecem no JSON e na tela. `/etc/os-release` fornece ID/ID_LIKE sem execução de seu conteúdo; binários absolutos permitem reconhecer instalações híbridas; ambas as famílias são reportadas, e ID/ID_LIKE definem a ordem de consulta. Root é recusado.

- RPM: motivo userinstalled do DNF (sem carregar repositórios ou configuração/plugins externos) não comprova qual pessoa instalou. Dependências ficam fora. Tamanho e INSTALLTIME vêm de RPM. SIZE vazio ou `(none)` fica sem tamanho; valor ilegível gera aviso daquele pacote e não tira a fonte do ar. Versões/arquiteturas simultaneamente instaladas podem gerar vários registros por nome. Protected.d ilegível, com links ou sintaxe não suportada protege conservadoramente todos os RPMs e gera aviso.
- APT: interseção apt-mark/dpkg, somente `install ok installed`; IDs multiarch usam binary:Package. Installed-Size vazio ou só com espaços é tamanho desconhecido (`null`), não erro da fonte. Valor não numérico ou estouro de KiB×1024 gera aviso daquele pacote e o item continua sem tamanho. KiB numérico vira bytes. Versão vazia fica `null`. Essential yes e Priority required/important são protegidos. Lê dpkg.log e rotações 1–12, simples e gzip, até 2 MiB por arquivo/descompressão. Datas usam timezone local atual, ignoram linhas inválidas/ambíguas e representam o primeiro install disponível, não necessariamente a instalação atual. Ausência de histórico resulta em null.
- Flatpak: apenas aplicativos, consultas separadas usuário/sistema; tamanho formatado é aproximado. Tamanho ausente fica desconhecido; texto ilegível vira nota do item e não falha a fonte. O mtime de ~/.var/app/ID tem confiança baixa e pode ser compartilhado entre instalações. Não prova execução nem inatividade.
- Snap: primeiro consulta HTTP GET local `/v2/snaps?select=all` em `/run/snapd.socket` (saída 8 MiB, prazo 20s, I/O 5s), sem autenticação privilegiada. Fallback fechado para CLI. Na API, tipo base/os/snapd é protegido, e tipo desconhecido também. Revisões mantidas separadamente, disabled aparece nas notas. A CLI não fornece tipo autoritativo para todas as bases: no fallback todos os snaps ficam conservadoramente protegidos, além do reconhecimento de core/base/snapd. Sem tamanho/data inventados. `installed-size` ausente fica desconhecido; valor ilegível vira nota e não falha a lista. Sem snapd socket a fonte fica indisponível.
- Steam: libraryfolders.vdf moderno e legado, appmanifest ACF; LastPlayed tem confiança média (metadado local editável). LastUpdated fica só nas notas. SizeOnDisk ausente fica desconhecido; valor ilegível vira nota e não invalida o jogo. Parser próprio: 2 MiB, profundidade 16, 30 mil nós, strings 16 KiB; máximo 64 bibliotecas e 20 mil entradas por biblioteca. Recusa caminhos relativos, `..`, symlinks inclusive ancestrais e arquivos não regulares. Bibliotecas legítimas via symlinks ficam parciais e exigirão desenho explícito futuro. Bibliotecas externas reais são aceitas, sem recursão nos jogos.

Tamanhos não são espaço recuperável nem somáveis como ganho físico. Nenhum sinal é prova de que seja seguro remover. IDs, nomes e notas são dados não confiáveis, escapados pela UI.

## Uso

`cargo run --locked -p inodune-cli --bin inodune -- apps list [--json]`

O binário legado `inodune-cli` aceita a mesma subordem. CLI retorna 3 quando há fonte parcial/erro; fontes ausentes são normais. Desktop: abrir “Programas & Jogos” e solicitar a listagem. A prévia web não lê inventário real.

## Proposta futura — não implementada nem autorizada

Para RPM/APT, usar PackageKit D-Bus: descobrir capacidades do backend, resolver nomes para IDs completos instalados com `Resolve`, verificar proteção e consultar dependências reversas; criar nova transação `RemovePackages` com flag bitfield `SIMULATE`. Mostrar integralmente as remoções/dependências propostas. Manter `autoremove=false`; dependências reversas inesperadas ou protegidas bloqueiam o plano. Falta de suporte à simulação também bloqueia.

Somente após nova autorização do usuário: revalidar IDs/versões/proteção, repetir simulação se houver mudança, pedir confirmação explícita do plano e deixar PackageKit/polkit mediar a operação real. Nunca usar sudo/pkexec ou converter um sinal de uso em autorização. Polkit não substitui confirmação do impacto. Falhas parciais precisam de relatório; rollback não é garantido. Flatpak e Snap usariam seus próprios backends, com revisão de instalação/revisão, dados persistentes, runtimes/bases compartilhados e políticas específicas.

Referências oficiais: [interface Transaction](https://www.freedesktop.org/software/PackageKit/doc/dbus-org.freedesktop.packagekit1.Transaction.html), [flags de transação](https://www.freedesktop.org/software/PackageKit/doc/enum.TransactionFlagEnum.html). A proposta depende das capacidades reais do backend de cada distro.

Próximos estudos: sinais KDE (atividade pode estar desabilitada), journald com escopo/permissões e retenção limitados; consentimento antes de ler históricos. Depois, pontuação explicável com proveniência, confiança e valores desconhecidos preservados, sem seleção ou ação automática.

## Verificação desta implementação

Base: `a80e5ee`, branch `feat/inodune-apps-readonly`, Rust 1.93.0; fontes novas e integrações CLI/Tauri/Svelte verificadas em conjunto na worktree.

- `cargo test --locked --offline --workspace --all-features`: passou; 14 testes do módulo e 9 testes do backend desktop. Os três testes especiais de mounts/ownership já ignorados pelo runner comum continuam ignorados; não foram certificados aqui.
- `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` e `cargo fmt --all --check`: passaram.
- `npm run typecheck`, `npm run lint`, `npm test` (13 testes) e `npm run build`: passaram.
- `npm run test:e2e -- --project=chromium`: 28 testes passaram, incluindo dois de inventário com IPC exclusivamente sintético. Capturas wide/narrow foram inspecionadas localmente. WebKit e GUI Tauri nativa interativa não foram executados nesta tarefa.
- RPM/DNF, Flatpak e Steam reais foram consultados localmente; somente contagens agregadas foram reportadas. O inventário real não foi salvo nem versionado.
- APT/dpkg, logs simples/gzip, Snap CLI/API e distros Ubuntu/Debian/derivadas: fixtures sintéticas. A execução Ubuntu real não foi validada: a infraestrutura existente usa Podman, cuja inspeção falhou com filesystem somente leitura em `/run/user/1000/libpod`. Não foram instalados pacotes ou modificados runtime/storage do host para contornar isso.
- A primeira execução Rust no sandbox falhou nas proteções existentes de armazenamento: o sandbox apresenta `/` e `/tmp` como pertencentes a nobody. Reexecutar com UID real e temporários privados da worktree passou, sem enfraquecer as proteções.
- Nenhum download foi necessário: crates e node_modules vieram de caches locais copiados para a worktree. Builds Cargo usaram `--offline`. Novas dependências diretas do módulo reutilizam versões do lock: serde, serde_json, rustix, chrono, flate2 e tempfile.

Estes são resultados locais, não aprovação humana, CI, validação Ubuntu nativa ou publicação.
