# Changelog

## 0.1.8

- Protect credential names and private-key signatures inside caches, including permanent cleanup.
- Send caches to Trash by default; restrict safe selection to low risk.
- Interpret PackageKit installation state using the signal enum, not opaque package data.
- Protect core utility providers and improve cleanup confirmation.
