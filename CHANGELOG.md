# Changelog

## 0.1.14

Polimento visual e de UX, com revisão independente (GPT-6.1 Terra no visual, GPT-6.1 Sol no
código e na segurança). Backend Rust e protocolo de limpeza inalterados.

### Corrigido

- **Tela em branco com dado inválido:** um resultado de análise fora do contrato (ID duplicado,
  tamanho fracionário ou acima do inteiro seguro, campo ausente) derrubava a interface inteira.
  Agora `validateScanResult` recusa o resultado antes de aplicá-lo: "O resultado da análise chegou
  em formato inesperado (…). Nenhum item foi aplicado e nada foi alterado." Uma seleção que não dá
  para estimar mostra "estimativa indisponível" (nunca "0 B") e bloqueia a limpeza. Ver ADR 0002.
- **Botão primário ilegível no hover:** o hover do botão secundário vencia por especificidade e
  deixava o rótulo com contraste de 1,1:1, inclusive em "Enviar N itens para a Lixeira".
- **Contraste e tema escuro:** borda de campos e botões com 3:1, texto de aviso de recuperação com
  `--warning-text`, anel do disco, aviso, contador e marca com cores próprias no escuro (antes o
  traço da marca quase sumia). Os dois blocos da paleta escura são conferidos por teste.
- **Histórico:** limpezas canceladas apareciam como "Incompleto". O detalhe mostra o estado lido
  do próprio relatório.
- **Paginação do relatório:** o seletor global `nav` deixava os botões de página em coluna.
- **Ordem de cabeçalhos e landmarks:** `h3` sem `h2` em Programas & Jogos; regiões de arquivos do
  relatório com o mesmo nome; "Itens protegidos10" como nome acessível.
- **Movimento reduzido:** o card ainda subia 2 px no hover; o arco parado do scanner parecia
  progresso determinado. Ambos somem com movimento reduzido.

### Alterado

- **Títulos funcionais:** cada página tem como título o nome da área (Desenvolvimento, Itens
  protegidos…) e um subtítulo concreto; o `<title>` acompanha. Saíram os eyebrows repetidos e as
  frases de rodapé.
- **Navegação:** Visão geral primeiro; Programas & Jogos com ícone próprio; a sidebar mostra o
  estado da análise (nenhuma, em andamento, concluída, parcial, desatualizada, cancelada, erro).
  Em janelas estreitas a faixa de áreas indica com seta e esmaecimento o lado que ainda tem itens.
- **Visão geral:** o anel mostra só ocupado e disponível medidos. "Selecionado para limpar" fica
  ao lado, como estimativa lógica, com a consequência da política (na Lixeira o espaço continua
  ocupado) e aviso de análise parcial perto do botão. O aviso de cobertura saiu do topo e virou
  nota informativa abaixo do painel. Ver ADR 0003.
- **Cards de categoria e de projeto:** mesma estrutura, título como link da área, barra de
  composição por risco (baixo risco, revisar, protegido) com legenda em texto. Só somam tamanhos
  conhecidos e sem sobreposição; desconhecidos aparecem como "Não medido", "≥ X" ou "N itens sem
  tamanho somado". Projetos ordenados pelo que pode ser limpo.
- **Lista de revisão:** risco sempre como badge com ícone; "Exige cuidado" vira nota adicional,
  sem esconder o risco real; grupos usam o risco mais cauteloso dos membros; detalhes com ícone
  próprio, separado do expandir. A barra de seleção fica fixa no rodapé em janelas ≥960×700.
- **"Limpar com 1 clique" → "Limpar lote seguro da análise"**, com a explicação de que independe
  da seleção atual, o destino (Lixeira ou exclusão permanente) e a confirmação do sistema a seguir.
- **"Por que este item apareceu?"** abre num painel lateral (o mesmo `<dialog>` modal), com
  tamanho e atividade no topo e motivo, efeito, preservação e recriação nessa ordem.
- **Simulador:** custos agrupados por detector, com os caminhos de cada grupo; painel de decisão
  fixo ao rolar em telas largas.
- **Relatório:** contadores por estado (na Lixeira, removidos, alterados e preservados, em
  quarentena, falharam, cancelados), itens com problema primeiro, sucessos recolhidos quando há
  mais de 12 arquivos, uma nota de recuperação por destino real; a variação de volume aparece em
  cor neutra.
- **Configurações:** reduzir animações e efeitos sonoros como switches (`role="switch"`).
- **Tipografia e tokens:** escala `--text-xs…--text-display`; ações e títulos de item em 14 px,
  metadados em 13 px; tracking dos títulos de −0,035em para −0,02em; tokens de gráfico, borda de
  controle, desabilitado, raios e easing.

- **Desempenho:** a lista da Visão geral só é montada quando aberta. Antes, milhares de linhas
  ocultas tornavam a tela lenta; no WebKit, 10.000 itens estouravam o tempo do teste, e 1.000 itens
  caíram de 18,2 s para 2,2 s.
- **Foco visível:** um item da lista que recebe foco rola para acima da barra de seleção fixa
  (WCAG 2.4.11). Só `scroll-padding` deixava até 27 de 56 paradas de foco cobertas no WebKit.

### Testes

- `tests/robustness.spec.ts`: resultados malformados recusados, soma fora do limite aceita mas sem
  limpeza, seleção sem estimativa bloqueada, protegidos sem tamanho como "Não medido".
- `tests/focus-not-obscured.spec.ts`: nenhuma parada de Tab termina atrás da barra fixa.
- `tests/grouped-rows.spec.ts`: fechar e reabrir a lista da Visão geral mantém os grupos
  expandidos (o estado fica no `App`, já que a lista só é montada quando aberta).
- O E2E grava capturas e amostras em `test-results/evidence`; `INODUNE_EVIDENCE=1` regenera
  `docs/evidence`. O nome do screenshot do relatório de limpeza segue o navegador.
- `scripts/native-automatic-e2e.py` atualizado para a interface e as fixtures atuais (já estava
  desatualizado na 0.1.13), com `INODUNE_EVIDENCE_DIR` para execuções de verificação.
- `model.test.ts`: contrato IPC completo, `trySimulateBudget`, `measuredSum`; `tokens.test.ts`:
  paletas escuras idênticas.

## 0.1.13

### Corrigido

- **Segurança — Lixeira sem origem válida:** uma pasta na Lixeira sem `.trashinfo` válido
  (ausente, `Path=` relativo, vazio ou com `.`/`..`) virava `trash.entry` de risco baixo,
  entrava no `--safe` e foi apagada com `--safe --permanent` (era o conteúdo de um `~/.ssh`).
  Agora fica só para análise ("Sem registro de origem válido na Lixeira; só análise."). Uma
  entrada cujo `Path=` passa por uma pasta sensível (`.ssh`, `.gnupg`, `.aws`, `.kube`,
  `.docker`, `.password-store`, `.config/gh`, `.config/gcloud`, `.azure`, `.terraform.d`,
  `.netrc`, `.git-credentials`, `.local/share/keyrings`, `.mozilla`, `.thunderbird`) também.
  O executor revalida a origem antes de cada arquivo.
- **Esvaziar a Lixeira:** no modo Lixeira, itens da Lixeira não voltam para a Lixeira (antes
  eram só renomeados e o item terminava incompleto). O plano recusa ("Itens da Lixeira só saem
  com exclusão permanente; use --permanent."); o `--safe` os deixa de fora e avisa quando só
  restavam eles; o app faz o mesmo. Com `--permanent`, os arquivos saem, depois as pastas
  vazias e a pasta da entrada (mesma identidade, sem seguir links) e só então o `.trashinfo`.
  Se algo protegido ou novo ficar, a pasta e o `.trashinfo` ficam juntos.
- **Aprovar pasta:** erros crus viram frases: "Pasta do sistema ou de outro usuário; escolha
  uma pasta sua.", "A pasta não existe.", "É um arquivo, não uma pasta." e "O caminho passa
  por um link; escolha a pasta real, sem links.". No app, aparecem num aviso âmbar junto aos
  botões de pasta, não na faixa vermelha de erro.
- **PackageKit:** "would remove dependent package(s)" vira "Outros pacotes do sistema dependem
  deste; o PackageKit recusou a remoção. Nenhum pacote foi removido.".
  `python3`, `python3-minimal` e `python3-libs` entram na lista de bloqueio.
- **Dependências Node agrupadas:** o agrupamento da 0.1.12 só existia no resumo da visão
  geral; as listas Desenvolvimento e Revisar mostravam uma linha por `node_modules`. Agora
  todas mostram um grupo que expande e lista cada pasta, com caminho, tamanho, seleção e
  "Por que…" próprios.
- **Textos:** "resta 1 item protegido, que fica" / "restam 4 itens protegidos, que ficam";
  motivo curto para credenciais preservadas ("Contém credencial ou não pôde ser lido; o
  arquivo e a pasta dele ficam fora da limpeza.").
- **Avisos repetidos:** um caminho negado e indisponível na descoberta aparece uma vez só.
- **Journal:** caminhos gravados sem `/./`; a identidade verificada não muda.
- **Man pages:** "VEJA TAMBÉM" não lista a própria página.

## 0.1.12

### Corrigido

- **Segurança — Lixeira e caches com o mesmo veredito:** a análise da Lixeira marcava
  arquivos de biblioteca (`.d.ts` do `@aws-sdk` com `aws_secret_access_key: string;`,
  `@smithy` dist-cjs, `lib.dom.d.ts`, `scanner.js`) como credencial, porque avaliava o nome
  aleatório `Trash/files/inodune-…`, sem extensão. Agora o `Path=` do `.trashinfo` é usado
  como dica de nome **não confiável**: leitura limitada, privada, com um único `Path=` válido,
  absoluto e sem componentes `.`, `..` ou vazios; ele só alimenta a mesma
  `credential_bytes` do `node_modules`. Cabeçalho de chave privada e valores reais protegem
  sempre; as regras de nome valem para o nome na Lixeira **e** para o nome original (mais
  estrito: um `.env` na Lixeira passa a ser protegido pelo nome original). Sem `.trashinfo`
  válido, a avaliação continua a mais protetora. O executor revalida da mesma forma.
- **EXDEV e outros errno:** "Invalid cross-device link (os error 18)" vira "Entrada em outro
  sistema de arquivos (ponto de montagem); não examinada."; todos os avisos da descoberta
  passam pelo tradutor, que cobre mais códigos. A entrada continua não examinada.
- **Cache só com pastas vazias:** depois de limpar o pip, a análise dizia "Cobertura
  incompleta". Agora diz "Nada a limpar: restam só pastas vazias." (evidência
  `nothing-to-clean`). O item continua não limpável e as pastas vazias não são removidas.
- **Desktop sem DRI3:** o binário define `WEBKIT_DISABLE_DMABUF_RENDERER=1` só quando a
  variável não existe, como primeira instrução de `main` (antes de threads, GTK e WebKit).
  Para reverter: `WEBKIT_DISABLE_DMABUF_RENDERER=0`. Detalhes em `docs/packaging-linux.md`.
- O teto de 4 MiB do relatório salvo vale para o arquivo serializado inteiro.
- Sem `/./` nas linhas recusadas e nos avisos do relatório.
- Recusas de permissão dizem "Arquivo" ou "Pasta" e citam o caminho.
- `--review` sem candidato elegível repete o motivo de proteção de cada candidato.
- GUI: a linha agrupada (ex.: "Dependências Node · 2 pastas") expande e lista cada pasta.
- GUI: cancelar uma desinstalação sem remover nada mostra um aviso neutro, não um alerta.
- Man pages em português (opções, arquivos, ambiente).
- Todo processo `git` da análise é aguardado, inclusive em retornos antecipados.
- Texto "Nada elegível para limpar" sem repetição.

## 0.1.11

### Corrigido

- **Segurança:** um cabeçalho `-----BEGIN … PRIVATE KEY-----` no início de linha protege
  sempre, inclusive em README, CHANGELOG e docs dentro de `node_modules`,
  `site-packages` e do registry do Cargo, e em METADATA de wheel. A exceção de
  documentação vale só para as regras de `aws_secret_access_key`, `_authToken` e netrc.
- **Evidência gravável pelo grupo (umask 002 do Ubuntu):** `Cargo.toml` 664 deixou de
  recusar `--review rust.target`. Uma regra única (`access::evidence_mode_allowed`) vale
  para todas as evidências: o+w sempre recusado; g+w só quando o arquivo é do usuário e o
  grupo é o grupo privado dele (gid primário, mesmo nome, sem outros membros nem outros
  usuários com esse gid primário). Erro de consulta recusa. O grupo privado agora também
  exige nome igual ao do usuário.
- **Hardlinks na Lixeira:** elos que foram juntos para a Lixeira (node-gyp) não deixam
  mais a análise incompleta; elos com algum link fora da Lixeira continuam gerando aviso
  `hardlink` e bloqueando. Toda recusa por análise incompleta lista os motivos (contagem
  por código e caminhos de exemplo). Análise incompleta ou cancelada continua recusando
  todo plano (`--review`, `--safe` e o padrão), como na 0.1.10.
- Sem "Not a directory (os error 20)" no motivo de arquivos protegidos (`.env`, `tfstate`,
  `id_ed25519`).
- `clean --permanent --dry-run`: ação, risco e impacto do item seguem a ação do plano, em
  português e com tamanhos legíveis.
- Relatório e registro final do journal sempre gravados: a cópia salva fica limitada,
  lista falhas e itens preservados primeiro e informa quantas linhas foram omitidas; os
  totais ficam completos.
- Um cache que só tem a parte preservada diz "Nada a limpar" em vez de "Cobertura
  incompleta"; plurais corretos ("1 item protegido").
- `--review` limpa todos os candidatos do detector num único plano (use `--root` para
  restringir).
- Sem `/./` no JSON de scan e de plano; a identidade do plano compara o caminho
  normalizado nos dois lados.
- `clean --yes` sem nada elegível responde `nothing_eligible` em vez de `dry_run`.
- Diálogos nativos com "Limpar"/"Desinstalar"/"Cancelar".
- Declarações TypeScript (`aws_secret_access_key: string;` em `.d.ts`) não são
  credenciais; valores reais em `.ts` continuam protegidos.
- Pacote: descrição longa abaixo de 80 colunas e man pages para os três binários.

## 0.1.10

### Corrigido

- **Segurança (regressão da 0.1.9):** credenciais detectadas pelo conteúdo voltaram a
  ficar protegidas em todos os alvos de limpeza (cache pip, `_cacache` do npm, `target`
  do Cargo, `node_modules`, Next.js, caches Python e demais), na Lixeira e na exclusão
  permanente, na CLI e na GUI. A 0.1.9 exigia um bloco PEM completo com DER/OpenSSH
  coerente, tratava como placeholder qualquer valor que *contivesse* `FAKE`, `EXAMPLE`,
  `XXX` ou `<`, exigia 20+ caracteres, não reconhecia linhas netrc e lia só 4 KiB.
  Agora uma política única (`inodune-core/src/credentials.rs`), usada pelo scan e pelo
  executor imediatamente antes de mover ou apagar cada arquivo, protege:
  - linha que começa com `-----BEGIN … PRIVATE KEY-----` (sem exigir corpo válido);
  - `_authToken` e `aws_secret_access_key` com valor que não seja placeholder;
  - entradas netrc (`machine`/`default`) com `password` preenchido.
  Placeholder é só o valor inteiro óbvio (vazio, `YOUR_…`, `<…>`, `${…}`/`$VAR`,
  `xxx`/`***`, `changeme`, `example`, `placeholder`, `dummy`, `redacted`). Leitura de
  64 KiB por arquivo e até 1 MiB em arquivos de texto; arquivo ilegível ou instável
  continua protegido. Exceções estreitas: METADATA/PKG-INFO/RECORD de wheels,
  README/CHANGELOG e docs dentro de pacotes, PEM dentro de string literal e
  `_authToken` usado como identificador. Só o arquivo e seu diretório imediato ficam
  preservados; o restante do cache é limpo. Nenhuma outra proteção foi afrouxada.
- CLI: `--review` aceita `node.modules` (e `node_modules`/`node-modules`) e lista os ids
  aceitos no erro; saída com "1 item", tamanhos legíveis, volume pelo ponto de montagem,
  erros do sistema em português e caminhos sem `/./` (também no `Path=` da Lixeira).
- Pacote `.deb`: changelog e copyright no mesmo `/usr/share/doc/inodune/`; `SHA256SUMS`
  lista só os arquivos presentes na pasta de saída.
- Texto da ação: modal, diálogo nativo, progresso, resumo e histórico derivam da ação
  real do plano (`Action::label`/`consequence` e `actionLabel`). Uma limpeza para a
  Lixeira não aparece mais como "apagados direto" nem como "exclusão permanente".
- Diálogo nativo "Confirmar plano de limpeza": tamanhos legíveis, "1 item"/"N itens" e
  caminhos sem `/./` (layout e botões inalterados).
- Histórico: journals privados até 64 MiB (o mesmo teto do escritor) são lidos em
  streaming; erros com código próprio (`JOURNAL_TOO_LARGE: journal grande demais: X > Y`,
  `JOURNAL_UNSAFE`, `JOURNAL_CHANGED`…). Checagens de dono, tipo, modo 0600 e links
  mantidas.
- Hardlinks dentro do próprio item (node-gyp `build/Release` ↔ `obj.target`) são limpos
  nos dois modos; elos fora do item continuam preservados.
- Mensagens: falha do sandbox do Git (bubblewrap) explicada em vez de "Git retornou
  erro."; "(os error N)" traduzido nos avisos do scan, nos resultados e na CLI.

## 0.1.9

### UX

- Overview: low-risk, supported items come pre-selected after every non-cancelled
  automatic scan (review and protected items never are). The clean button shows the
  selected count and size ("Limpar 41 itens · 183,7 MiB") and is disabled with an
  in-place hint when nothing is selected; no more alert that shifts the layout.
- Cleanup confirmation is a focused modal with per-category totals instead of a block
  pushed below the list. Plan hash and snapshot id are no longer shown.
- Coverage notice: internal keys (`structural_denied`, `system_protected`, `hardlink`…)
  become Portuguese phrases, equivalent keys are merged, OS errors such as
  "Not a directory (os error 20)" are translated and grouped with sample paths. The
  notice appears only on the Overview (not on Settings or every page).
- Displayed paths drop redundant `/./` segments (lists, dialogs, report, History).
- Review list groups repeated detections (e.g. 40 × "Bytecode Python" in one row with a
  tri-state checkbox) and shows path and reason for protected rows.
- Report and History: sizes in KiB/MiB/GiB, correct singular/plural, volumes named by
  mount point instead of "Dispositivo 42", deduplicated and translated warnings, plan id
  moved to "Detalhes técnicos".
- Apps & Games: search, source/protection/size filters, sort and an "N de M" counter;
  protected rows show their reason inline. Uninstall confirmation is a focused modal with
  friendly name, version, dependencies removed with it, package count and installed size;
  raw PackageKit ids sit in a collapsed "Identificadores técnicos". Errors use alert
  styling with specific titles (polkit denied/cancelled, dependents, PackageKit
  unavailable or busy, protected package).
- PackageKit `ErrorCode` is mapped to specific messages (codes checked against
  PackageKitGlib) and is no longer overwritten by the generic `Finished` failure. The
  native dialog shows the same readable plan.
- Review items that can be cleaned after explicit selection no longer say "Inventário não
  autoriza remoção"; the duplicated "Cobertura incompleta (cobertura incompleta)" is gone.
  No protection was relaxed.
- Portuguese labels replace "SPACE BUDGET SIMULATOR", "PROJECT LENS" and
  "PROTECTION LEDGER" (English names remain in en-US).
- The demo corpus and demo-only UI ("Protótipo navegável", fictitious volume) are only in
  development/test builds; `vite build` resolves an empty corpus. Playwright builds a
  separate `dist-e2e` with `INODUNE_DEMO=1`.
- CLI: help cites `inodune-cli`, `clean --demo` works without `--dry-run`, volume changes
  print as `−32 KiB` instead of `Some(-32768)`, `apps list` prints readable sizes, and
  `apps uninstall` shows packages, dependencies and size before asking for
  `UNINSTALL N`.
- Debian package: Maintainer with a public contact, `Section: utils`, `Homepage`,
  `Depends: libc6 (>= 2.34), libgcc-s1`, a Debian changelog and a DEP-5 copyright file
  (lintian: malformed-contact, missing-dependency-on-libc, no-changelog,
  no-copyright-file).

### Backend

- Deduplicate identical PackageKit Resolve IDs; distinct identities remain ambiguous.
- Share inventory/protection policy between CLI and helper, including Essential,
  Priority required/important, core runtime libraries, utility owners and RPM protected.d.
  Requests outside the inventory or inconsistent primary/selected IDs are refused.
- Accept dpkg trigger progress only for packages already installed at that version;
  success additionally requires an exact before/after native package-state comparison.
  Planned versions must be removed; unrelated removals, additions or version changes
  remain errors with unknown outcome and no automatic retry.
- Replace credential substrings with exact names and structured, bounded content checks.
  Source literals and placeholders are not credentials. Complete private-key fixtures
  remain protected. Preserve each sensitive file and its immediate directory (or just
  the file when directly inside the cache root), excluding that subtree from the consent-bound
  manifest. Safe sibling files can go to Trash after review. The root cache directory
  and excluded subtrees are never removed wholesale, including permanent cleanup.
- Intentional discovery symlinks are skipped with a reason without blocking other caches.
  Unreadable areas, ownership/mount changes and incomplete manifests still fail closed.
- Clarify reviewed cache impact: eligible manifest files go to Trash by default.
- Generate the Rust SPDX SBOM and distribution SHA256SUMS inside the offline Debian 12
  packager; host Cargo is no longer required by package-linux.py. Outputs remain beside
  the distribution artifacts. Frontend SBOM generation uses host npm.

## 0.1.8

- Protect credential names and private-key signatures inside caches, including permanent cleanup.
- Send caches to Trash by default; restrict safe selection to low risk.
- Interpret PackageKit installation state using the signal enum, not opaque package data.
- Protect core utility providers and improve cleanup confirmation.
