# Programas & Jogos — inventário v0.1.6

O inventário é somente leitura e independente de `inodune-core::Candidate` e do executor de caches. Não há score de desuso ou persistência automática do inventário. A desinstalação possui fluxo separado de simulação, plano e confirmação nativa descrito abaixo.

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

## Desinstalação na 0.1.6

A listagem continua somente leitura. A nova ACL `allow-installed-apps-uninstall` concede somente `uninstall_prepare` e `uninstall_execute` à janela local main. O primeiro recebe fonte e ID estritamente validados; o segundo recebe apenas o ID opaco do plano. Não há plugin de shell ou comando arbitrário. A permissão de inventário continua separada.

O executável `inodune-uninstall-helper`, separado da GUI, recusa UID/EUID root. Não chama sudo, pkexec ou su, não lê nem solicita senhas. APT e RPM usam GIO D-Bus para `Resolve` instalado e `RemovePackages`, primeiro com o bit SIMULATE (1 << 2). `allow_deps` e `autoremove` são sempre false. A execução real permite autorização interativa do serviço pelo polkit. O app não manipula essa autorização.

O helper mantém um único plano em memória por até 120 segundos. A confirmação nativa mostra a lista completa. O plano é consumido inclusive em cancelamento/erro. Após a confirmação, repete Resolve e simulação, compara exatamente IDs e versões e verifica novamente as proteções antes de executar. Lista vazia, duplicada, ambígua, desconhecida, mudança ou expiração recusam a operação inteira. Essential/Priority required/important, protected.d, kernel e pacotes de sessão são bloqueados. Regras protected.d desconhecidas bloqueiam em vez de serem ignoradas.

IDs aceitam somente gramáticas ASCII por fonte, sem espaço, opções iniciadas por hífen, separadores de shell ou caminhos. Não há shell. Steam só abre `steam://uninstall/<appid>` numérico pelo xdg-open; a confirmação e a remoção pertencem ao Steam, e abrir a URI não prova desinstalação.

**Flatpak:** a libflatpak é carregada somente de caminhos absolutos do sistema. A instalação é escolhida como user ou system e o app deve ter branch/arquitetura inequívocas. Uma transação de uninstall é resolvida e sempre abortada em `ready-pre-auth`, antes de autorização/mutação. A lista inclui somente o app e extensões cujo metadado `ExtensionOf/ref` corresponde ao app exato; runtimes base e outros apps recusam o plano inteiro. Capturamos refs completos e commits e re-simulamos antes de executar. A execução usa outra transação da mesma libflatpak, com as mesmas flags (disable_dependencies, no_pull, disable_prune; refs relacionados habilitados). No ready-pre-auth da execução, exigimos a lista exata de refs e commits revisados, inclusive igualdade com o commit instalado, antes de retornar TRUE. Uma diferença aborta a própria transação antes do polkit. Não há fallback para CLI, unused ou delete-data. Sistema deixa a autorização com polkit. Sem biblioteca, sinal ou metadados suficientes, a preparação é bloqueada com motivo explícito.

**Snap:** consulta `/v2/snaps?select=all` e `/v2/connections?select=all`. Exige app ativo, revisão exata, bases conhecidas de outros apps e nenhum consumidor dos slots content do alvo. Tipos base/core/os/snapd/kernel/gadget, dependentes ou dados ausentes bloqueiam. A lista contém exatamente nome/revisão; após revalidar, faz POST `/v2/snaps/<nome>` com somente `{"action":"remove"}`, permite interação polkit pelo socket e acompanha a mudança até conclusão. Não usa purge. Se o snapd não expuser bases ou conexões completas, informa precisamente essa falta e bloqueia.

Testes usam mocks de transporte e fixtures; não chamam PackageKit/polkit nem removem pacotes reais do host. Um timeout em transação real significa resultado desconhecido, não rollback ou falha sem efeito. Não há retry automático. A re-simulação reduz TOCTOU, mas não fornece um bloqueio global contra outro gerenciador ou processo do mesmo usuário entre chamadas D-Bus. O backend continua responsável por dependências e autorização. No Flatpak, a comparação adicional dentro da transação real fecha a troca de lista entre simulação e execução pelo CLI; esse CLI não é mais usado para remover.

APIs consultadas: [PackageKit RemovePackages/Resolve](https://www.freedesktop.org/software/PackageKit/doc/dbus-org.freedesktop.packagekit1.Transaction.html), [flags de transação](https://www.freedesktop.org/software/PackageKit/doc/enum.TransactionFlagEnum.html), [snapd REST](https://snapcraft.io/docs/reference/development/snapd-rest-api/). GIO 0.18.4 e GLib 0.18.5 (MIT), e libloading 0.7.4 (ISC), já constavam no Cargo.lock; suas bibliotecas de sistema já integram as dependências do Tauri.

### Revisão de segurança antes dos pacotes 0.1.6

Depois de tentar enviar o ID para execução, o processo Tauri nunca mata o helper, nem em Drop, erro de canal ou timeout. Um reaper aguarda apenas esse filho. Planos não enviados continuam canceláveis. A espera Tauri é de 900 s. O helper anuncia resultado desconhecido após 600 s de execução (180 s para Flatpak), mas continua aguardando a chamada original: esses prazos notificam, não cancelam nem matam uma mutação. O usuário deve consultar o inventário e não repetir automaticamente.

Sinais PackageKit seguem o contrato legado `Package`: Resolve aceita somente INSTALLED=1; RemovePackages SIMULATE aceita somente REMOVING=13. Qualquer outro info recusa a lista inteira. O quarto campo do package_id deve ser `installed`, prefixo `installed:`, `@System` ou prefixo `@System:`. Outros dados, versões do protocolo ou listas incompletas bloqueiam; não interpretamos AVAILABLE como instalado. Os IDs completos, incluindo versão/arquitetura, da re-simulação são enviados sem alterações para a remoção real, com allow_deps=false e autoremove=false, cobertos por teste de argumentos.

Os bloqueios fixos incluem prefixos dos gerenciadores (dnf/libdnf/rpm/apt/dpkg/packagekit/flatpak/snapd), kernel, sessão, login, rede, D-Bus, PAM, shell e ferramentas básicas. Há regressões para dnf5, libdnf5 e gnome-session-*, além de fixtures de ciclo de vida que demonstram que um helper enviado continua vivo quando a GUI deixa de aguardá-lo.
