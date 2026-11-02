# Changelog

## 0.1.9

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
