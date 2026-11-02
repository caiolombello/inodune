# Changelog

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
