<script lang="ts">
  import { t, formatNumber } from './i18n.svelte';
  import type { CleanupFileResult } from './backend';
  import RecoveryGuide from './RecoveryGuide.svelte';
  import { formatBytes } from './model';
  import { displayPath, systemMessage } from './present';

  let {
    files,
    label = '',
    compact = false,
  }: {
    files: CleanupFileResult[];
    label?: string;
    /** Report view: the page shows one recovery note for Trash/removed. */
    compact?: boolean;
  } = $props();
  let page = $state(0);
  const pageSize = 100;
  const pageCount = $derived(Math.max(1, Math.ceil(files.length / pageSize)));
  const visible = $derived(files.slice(page * pageSize, (page + 1) * pageSize));
  const statusLabel: Record<string, string> = {
    trashed: 'Movido para a Lixeira',
    removed: 'Removido',
    skipped_changed: 'Alterado e preservado',
    failed: 'Falha; preservado',
    cancelled: 'Cancelado; preservado',
    quarantined: 'Quarentena; preservado',
  };
</script>

<section
  class="outcomes"
  aria-label={label
    ? `${t('Arquivos de')} ${t(label)}`
    : t('Arquivos do relatório')}
>
  {#if files.length === 0}<p class="muted">
      {t('Este relatório não contém linhas de arquivo.')}
    </p>{:else}
    <div class="rows">
      {#each visible as file, index (page * pageSize + index)}
        <article class="row">
          <div class="row-top">
            <strong>{t(statusLabel[file.status] ?? file.status)}</strong><span
              >{formatBytes(file.logical_bytes)}</span
            >
          </div>
          <code class="path">{displayPath(file.display_path)}</code>
          {#if file.message}<p>{systemMessage(file.message)}</p>{/if}
          {#if !compact || (file.status !== 'trashed' && file.status !== 'removed')}<RecoveryGuide
              action={null}
              {file}
            />{/if}
          {#if file.preserved_at && file.status !== 'quarantined'}<p>
              {t('Preservado em')}:
              <code>{displayPath(file.preserved_at)}</code>
            </p>{/if}
          {#if !compact}<small
              >{t('Durabilidade confirmada')}: {'durability_confirmed' in
                file && file.durability_confirmed
                ? t('Sim')
                : t('Não')}</small
            >{:else if !('durability_confirmed' in file && file.durability_confirmed)}<small
              class="durability-warning"
              >{t('Durabilidade não confirmada: confira este arquivo.')}</small
            >{/if}
        </article>
      {/each}
    </div>
    {#if pageCount > 1}<nav class="pages" aria-label={t('Paginação')}>
        <button disabled={page === 0} onclick={() => page--}
          >{t('Anterior')}</button
        >
        <span>{formatNumber(page + 1)} / {formatNumber(pageCount)}</span>
        <button disabled={page + 1 >= pageCount} onclick={() => page++}
          >{t('Próxima')}</button
        >
      </nav>{/if}
  {/if}
</section>

<style>
  .rows {
    display: grid;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-inner);
    background: var(--surface);
    overflow: hidden;
  }
  .row {
    padding: 10px 14px;
    font-size: var(--text-sm);
  }
  .row + .row {
    border-top: 1px solid var(--stroke);
  }
  .row-top {
    display: flex;
    justify-content: space-between;
    gap: 16px;
  }
  .row-top strong {
    font-weight: 600;
  }
  .row-top span {
    font-variant-numeric: tabular-nums;
    color: var(--secondary-text);
  }
  .row p {
    margin: 4px 0;
    color: var(--secondary-text);
  }
  .path {
    display: block;
    overflow-wrap: anywhere;
    margin-top: 2px;
    font-size: var(--text-sm);
    color: var(--secondary-text);
  }
  small,
  .muted {
    color: var(--muted);
  }
  .pages {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 0.8rem;
  }
  .durability-warning {
    color: var(--warning-text);
  }
  .pages button {
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--control-border);
    border-radius: var(--radius-control);
    padding: 6px 12px;
    min-height: 32px;
  }
  .pages button:disabled {
    color: var(--disabled-text);
    background: var(--disabled-bg);
    border-color: var(--stroke);
  }
</style>
