<script lang="ts">
  import { CircleAlert, ChevronDown } from '@lucide/svelte';
  import type { NativeResult } from './backend';
  import { formatNumber, t } from './i18n.svelte';
  import { countLabel, groupIssues, groupSkipReasons } from './present';

  let { result }: { result: NativeResult } = $props();
  const reasons = $derived(groupSkipReasons(result.stats.skip_reasons));
  const issues = $derived(groupIssues(result.issues));
</script>

<section class="coverage-notice" aria-labelledby="coverage-title">
  <CircleAlert size={20} aria-hidden="true" />
  <div class="coverage-body">
    <h2 id="coverage-title">{t('Algumas áreas ficaram fora da análise')}</h2>
    <p>
      {countLabel(
        result.stats.skipped,
        'entrada não foi examinada.',
        'entradas não foram examinadas.',
      )}
      {t(
        'Itens incertos continuam protegidos, e a limpeza só altera o que estiver no plano que você confirmar.',
      )}
    </p>
    {#if reasons.length}<ul class="coverage-reasons">
        {#each reasons as reason (reason.label)}<li>
            <span>{reason.label}</span><strong
              >{formatNumber(reason.count)}</strong
            >
          </li>{/each}
      </ul>{/if}
    {#if issues.length}<details>
        <summary
          ><ChevronDown size={15} aria-hidden="true" />
          {t('Ver caminhos afetados')} ({formatNumber(
            result.issues.length,
          )})</summary
        >
        {#each issues as group (group.label)}<div class="issue-group">
            <strong>{group.label} · {formatNumber(group.count)}</strong>
            <ul>
              {#each group.paths as path, index (index)}<li>
                  <code>{path}</code>
                </li>{/each}
            </ul>
            {#if group.count > group.paths.length}<small
                >{t('e mais')}
                {countLabel(
                  group.count - group.paths.length,
                  'caminho',
                  'caminhos',
                )}</small
              >{/if}
          </div>{/each}
      </details>{/if}
  </div>
</section>

<style>
  /* Partial coverage is normal for an automatic scan: an informative note,
     not an alarm. The amber edge and icon keep it noticeable. */
  .coverage-notice {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    background: var(--surface);
    color: var(--secondary-text);
    border: 1px solid var(--stroke);
    border-left: 3px solid var(--warning);
    border-radius: var(--radius-inner);
    padding: 14px 16px;
    margin-bottom: 20px;
  }
  .coverage-notice > :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--warning-text);
  }
  .coverage-body {
    min-width: 0;
    flex: 1;
  }
  h2 {
    font-size: var(--text-md);
    color: var(--text);
    margin: 0 0 2px;
    letter-spacing: 0;
  }
  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  .coverage-reasons {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .coverage-reasons li {
    display: inline-flex;
    gap: 8px;
    align-items: baseline;
    font-size: var(--text-xs);
    padding: 3px 8px;
    border-radius: var(--radius-badge);
    background: var(--subtle);
    color: var(--secondary-text);
  }
  .coverage-reasons strong {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  details {
    margin-top: 8px;
    font-size: var(--text-sm);
    border: 0;
    padding: 0;
    background: none;
  }
  summary {
    cursor: pointer;
    display: inline-flex;
    gap: 6px;
    align-items: center;
    font-weight: 600;
    color: var(--accent);
    min-height: 28px;
  }
  details[open] summary :global(svg) {
    transform: rotate(180deg);
  }
  .issue-group {
    margin-top: 10px;
  }
  .issue-group ul {
    list-style: none;
    padding: 0;
    margin: 4px 0;
  }
  .issue-group code {
    overflow-wrap: anywhere;
    color: var(--text);
  }
</style>
