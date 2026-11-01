# Dependências — F4, 01/11/2026

Consulta local, separada de testes, revisão de código e aprovação de release. Nenhuma supressão transforma aviso em correção.

- `npm audit --json --fetch-timeout=30000 --fetch-retries=0`: 14 registros, 2 low e 12 moderate em dependências de desenvolvimento. `npm audit --omit=dev` reportou zero. [JSON completo](npm-audit.json), [produção](npm-production-audit.json). A primeira chamada sem timeout demorou e não foi tomada como resultado; as consultas limitadas terminaram. Sem downgrade forçado indicado como “fix”.
- `cargo-audit 0.22.2`, DB RustSec `2295466c4e9df26647fd67d8fa1c705ac2fc7f81` (atualização 30/10/2026): zero entradas em `vulnerabilities`, mas **dois avisos**. [JSON](rustsec-audit.json). Não interpretar “zero vulnerabilities” como ausência de risco.

| Aviso | Alcance e decisão |
| --- | --- |
| [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), glib 0.18.5, `VariantStrIter` unsound | `tauri 2.12.5 → gtk 0.18.2 → glib 0.18.5` e WebKitGTK compartilham a série. Correção publicada requer glib ≥0.20, incompatível com `gtk ^0.18`; forçar dependência direta cria outra família de tipos. A proposta de backport [PR 2009](https://github.com/gtk-rs/gtk-rs-core/pull/2009) foi fechada sem merge; [discussão de release](https://github.com/gtk-rs/gtk-rs-core/issues/2010). Não foi encontrada chamada `array_iter_str` fora da implementação/docs/testes nos fontes baixados, mas isso não prova inacessibilidade universal. Mantido lockfile, aviso aberto e revisão humana pré-release; sem fork silencioso ou alegação de corrigido. |
| [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html), proc-macro-error 1.0.4 sem manutenção | Entradas inversas são `glib-macros` e `gtk3-macros`, de compilação. Sem substituto compatível direto no grafo atual. Risco de manutenção registrado separadamente; não é evidência de exploração no binário. |

Repetir auditoria antes de qualquer publicação, revisar novos avisos e acompanhar uma cadeia Tauri/GTK suportada que remova os avisos. SPDX e SHA256SUMS são inventários/checksums locais, não assinatura nem prova de alcance de vulnerabilidades. CI remoto, auditoria humana e validação de produção não foram executados.
