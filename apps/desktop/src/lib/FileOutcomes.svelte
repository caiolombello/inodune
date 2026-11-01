<script lang="ts">
  import { t, formatNumber } from './i18n.svelte';
  import type { CleanupFileResult } from './backend';

  let { files }: { files: CleanupFileResult[] } = $props();
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

<section class="outcomes" aria-label={t('Arquivos do relatório')}>
  {#if files.length === 0}<p class="muted">
      {t('Este relatório não contém linhas de arquivo.')}
    </p>{:else}
    <div class="rows">
      {#each visible as file, index (page * pageSize + index)}
        <article class="row">
          <div class="row-top">
            <strong>{t(statusLabel[file.status] ?? file.status)}</strong><span
              >{formatNumber(file.logical_bytes)} {t('bytes lógicos')}</span
            >
          </div>
          <code class="path">{file.display_path}</code>
          <p>{file.message ?? t('Sem mensagem registrada.')}</p>
          {#if file.preserved_at}<p>
              {t('Preservado em')}: <code>{file.preserved_at}</code>
            </p>{/if}
          <small
            >{t('Durabilidade confirmada')}: {'durability_confirmed' in file &&
            file.durability_confirmed
              ? t('Sim')
              : t('Não')}</small
          >
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
    gap: 0.65rem;
  }
  .row {
    padding: 0.8rem;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-md, 12px);
    background: var(--surface-raised, var(--surface));
  }
  .row-top {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
  }
  .row p {
    margin: 0.45rem 0;
    color: var(--secondary-text);
  }
  .path {
    display: block;
    overflow-wrap: anywhere;
    margin-top: 0.35rem;
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
  .pages button {
    color: var(--text);
    background: var(--surface-raised, var(--surface));
    border: 1px solid var(--stroke);
    border-radius: 8px;
    padding: 0.4rem 0.7rem;
  }
  .pages button:disabled {
    opacity: 0.5;
  }
</style>
