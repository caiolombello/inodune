# Changelog

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
