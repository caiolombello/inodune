# Scanner benchmark — Fase 4

Executado em 2026-11-01T11:48:35Z por `python3 scripts/benchmark-fixtures.py`, somente leitura em árvores Cargo privadas de TemporaryDirectory. Nenhuma limpeza executada. Binário debug CLI SHA-256 `f50411edba1a0c053c701b4e0ab6e73a64203c105c30b016c18b1f3fb2e2032d`; fonte e ambiente completos no [JSON](scanner-benchmark.json). O JSON anterior ao marcador de panic está preservado em `scanner-benchmark-before-diagnostics.json`.

| Candidatos | Encontrados | Entradas visitadas | Parede (s) | Pico RSS (KiB) | Cobertura |
| ---: | ---: | ---: | ---: | ---: | --- |
| 100 | 100 | 500 | 0.015542 | 5,108 | Completa |
| 1,000 | 1,000 | 5,000 | 0.126439 | 8,640 | Completa |
| 10,000 | 10,000 | 50,000 | 1.217540 | 43,172 | Completa |

Tempos incluem startup e serialização JSON. Não representam performance do Svelte, ganho de caching entre scans ou hardware desktop mediano. [UI 100/1.000/10.000](ui-benchmark.md) foi medida separadamente com IPC sintético; 60 fps nativo em hardware representativo e participantes seguem pendentes.
