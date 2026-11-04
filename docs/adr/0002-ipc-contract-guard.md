# ADR 0002 — Contrato IPC validado e erros de estimativa contidos na interface

Status: aceito, 03/11/2026 (0.1.14).

## Contexto

O resultado da análise chega do backend Rust pelo IPC do Tauri e alimenta cálculos derivados (`$derived`) da interface. `simulateBudget` (`apps/desktop/src/lib/model.ts`) lança exceção para IDs duplicados, cobertura nula ou sobreposta e soma fora de `Number.MAX_SAFE_INTEGER`. Lançada dentro de um `$derived` do `App.svelte`, a exceção deixava a área principal vazia, a navegação com estado obsoleto e só um recarregamento recuperava a tela. Um `<svelte:boundary>` no próprio `App.svelte` não captura essa exceção: no Svelte 5.57 o erro é encaminhado a partir do efeito dono do `$derived`, não do leitor (verificado com protótipo na auditoria de 03/11).

O produtor Rust garante os invariantes no caminho comum. O risco é deriva de contrato entre versões ou um bug futuro. Quando acontece, a falha é total e silenciosa.

## Decisão

1. **Fronteira IPC:** `validateScanResult` confere todos os campos que a interface lê, antes de aplicar o resultado:
   - no resultado: `scan_id`, `complete`/`cancelled`, `issues`, `volumes`, `stats` e `skip_reasons`;
   - em cada candidato: ID não vazio e único, `logical_bytes` e `allocated_bytes_estimate` inteiros seguros, `risk` e `supported_action` conhecidos, todos os textos (nome, caminho, detector, categoria, projeto, motivo, impacto, recriação, preservação, atividade), `coverage_key` string ou nulo, `selected_by_default` booleano e `evidence` como lista de strings.

   Os tipos seguem `Candidate`, `ScanResult`, `VolumeInfo`, `ScanIssue` e `ScanStats` do Rust. Um resultado inválido nunca entra no estado. A análise termina em erro, com o motivo e a frase "Nenhum item foi aplicado e nada foi alterado".
2. **Estimativa como dado:** `trySimulateBudget` devolve `{ ok: false, error }` em vez de lançar. Com erro, a interface mostra "estimativa indisponível" (nunca "0 B") e um aviso. `askClean`, `cleanSafe`, `prepareLocalCleanup` e `startExecution` recusam enquanto a estimativa estiver inválida.
3. **Totais de exibição:** `measuredSum` soma só tamanhos com `coverage_key` conhecido, cada cobertura uma vez, e conta à parte os desconhecidos. Os totais aparecem como "≥ X" ou com "N itens sem tamanho somado", nunca tratando desconhecido como zero.

O backend continua sendo a autoridade: a validação do frontend é defesa em profundidade e não substitui a revalidação do plano no executor.

**Limites numéricos (u64 → número JS):** o Rust usa `u64`; acima de `Number.MAX_SAFE_INTEGER` (≈ 8 PiB) o JSON perde precisão. Tamanhos de candidatos entram em estimativas e planos, então precisam ser inteiros seguros: um candidato acima disso recusa o resultado. Tamanhos de volume e contagens de estatística só são exibidos e aceitam qualquer número finito não negativo, mostrado de forma aproximada. Somas não são limitadas na validação; a soma da seleção é conferida por `simulateBudget`, que bloqueia a limpeza com "estimativa indisponível". `stats.skip_reasons` tem `#[serde(default)]` e pode faltar; quando presente, precisa ser um mapa de contagens.

## Alternativas

- **`<svelte:boundary>` em volta das páginas:** só é eficaz depois de mover o estado para um controlador em nível de módulo (`.svelte.ts`). Fica como passo da decomposição do `App.svelte`, não como correção imediata.
- **Boundary em volta de `<App/>`:** substituiria o app inteiro e interromperia o polling de uma limpeza em andamento.

## Consequências

- `tests/robustness.spec.ts` cobre:
  - recusa de IDs duplicados, tamanho fracionário, tamanho acima do inteiro seguro e campo ausente;
  - soma fora do limite aceita, mas sem limpeza;
  - seleção sem cobertura;
  - protegidos sem tamanho exibidos como "Não medido".

  Nenhum caso pode gerar `pageerror` nem chamar `prepare_cleanup`.
- `model.test.ts` cobre `validateScanResult`, `trySimulateBudget` e `measuredSum`.
- Se o backend passar a emitir um novo `risk` ou `supported_action`, a análise falha de forma visível até o contrato do frontend ser atualizado. É o comportamento desejado para um limpador de arquivos.
