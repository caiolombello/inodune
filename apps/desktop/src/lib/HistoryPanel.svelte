<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { formatDate, language, t } from './i18n.svelte';
  import FileOutcomes from './FileOutcomes.svelte';
  import RecoveryGuide from './RecoveryGuide.svelte';
  import type { CleanupExecutionReport } from './backend';
  import { formatBytes } from './model';
  import { actionLabel, countLabel, displayPath } from './present';

  type Entry = {
    report_id: string;
    created_unix: number;
    plan_id: string;
    action: 'trash' | 'permanent' | null;
    complete: boolean;
    cancelled: boolean;
    logical_bytes_processed: number;
    files_count: number;
    error: string | null;
  };
  let { refreshKey = '' }: { refreshKey?: string } = $props();
  let entries = $state<Entry[]>([]);
  let selected = $state<Entry | null>(null);
  let report = $state<CleanupExecutionReport | null>(null);
  let error = $state('');
  let loading = $state(false);
  let reportLoading = $state(false);
  let exporting = $state(false);
  let exportedPath = $state('');

  async function refresh() {
    loading = true;
    error = '';
    try {
      entries = await invoke<Entry[]>('list_history');
    } catch (e) {
      entries = [];
      error = String(e);
    } finally {
      loading = false;
    }
  }
  async function open(entry: Entry) {
    selected = entry;
    report = null;
    error = '';
    reportLoading = true;
    exportedPath = '';
    try {
      const value = await invoke<CleanupExecutionReport>('get_history_report', {
        reportId: entry.report_id,
      });
      if (selected?.report_id === entry.report_id) report = value;
    } catch (e) {
      if (selected?.report_id === entry.report_id) error = String(e);
    } finally {
      if (selected?.report_id === entry.report_id) reportLoading = false;
    }
  }
  async function exportSelected() {
    if (!selected || !report || exporting) return;
    exporting = true;
    error = '';
    exportedPath = '';
    try {
      const path = await invoke<string | null>('export_history_report', {
        reportId: selected.report_id,
        locale: language.value,
      });
      if (path) exportedPath = path;
    } catch (e) {
      error = String(e);
    } finally {
      exporting = false;
    }
  }
  $effect(() => {
    if (typeof refreshKey === 'string') void refresh();
  });
</script>

<section class="history">
  <div class="heading">
    <div>
      <h2>{t('Histórico local')}</h2>
      <p>
        {t(
          'Relatórios salvos no dispositivo. Os caminhos exibidos vêm dos relatórios persistidos.',
        )}
      </p>
    </div>
    <button onclick={refresh} disabled={loading}
      >{loading ? t('Carregando…') : t('Atualizar')}</button
    >
  </div>
  {#if error}<p class="error" role="alert">{t('Erro')}: {error}</p>{/if}
  {#if !loading && entries.length === 0 && !error}<div class="empty">
      <h3>{t('Nenhum relatório salvo')}</h3>
      <p>{t('Relatórios finalizados aparecerão aqui.')}</p>
    </div>{/if}
  {#if entries.length}<div class="layout">
      <ul class="entries">
        {#each entries as entry (entry.report_id)}<li>
            <button
              class:selected={selected?.report_id === entry.report_id}
              onclick={() => open(entry)}
            >
              <span class="date"
                >{entry.created_unix
                  ? formatDate(entry.created_unix * 1000)
                  : t('Data indisponível')}</span
              >
              {#if entry.error}<strong>{t('Relatório com erro')}</strong
                >{:else}<strong
                  >{actionLabel(entry.action)} · {entry.complete
                    ? t('Concluído')
                    : t('Incompleto')}</strong
                >{/if}
              {#if !entry.error}<span
                  >{countLabel(entry.files_count, 'arquivo', 'arquivos')} · {formatBytes(
                    entry.logical_bytes_processed,
                  )}
                  {t('processados')}</span
                >{/if}
              {#if entry.error}<span class="error"
                  >{t('Relatório indisponível')}: {entry.error}</span
                >{/if}
            </button>
          </li>{/each}
      </ul>
      <div class="detail">
        {#if selected && report}
          <header>
            <h3>{t('Detalhes do relatório')}</h3>
            <p>{t(report.measurement_note)}</p>
            <details class="technical">
              <summary>{t('Detalhes técnicos')}</summary>
              <p>{t('Plano')}: <code>{report.plan_id}</code></p>
            </details>
            <RecoveryGuide
              action={report.action}
              complete={report.complete}
              cancelled={report.cancelled}
              warnings={report.warnings}
            />
            <button onclick={exportSelected} disabled={exporting}>
              {exporting ? t('Exportando…') : t('Exportar JSON local')}
            </button>
            <p>
              {t(
                'O relatório inclui caminhos locais. Arquivos existentes nunca são substituídos.',
              )}
            </p>
            {#if exportedPath}<p role="status">
                {t('Relatório exportado')}:
                <code>{displayPath(exportedPath)}</code>
              </p>{/if}
          </header>
          <FileOutcomes files={report.items.flatMap((item) => item.files)} />
        {:else if selected}<p>
            {reportLoading
              ? t('Carregando relatório…')
              : t('Relatório indisponível')}
          </p>{:else}<p>
            {t('Selecione um relatório para ver os detalhes.')}
          </p>{/if}
      </div>
    </div>{/if}
</section>

<style>
  .history {
    display: grid;
    gap: 1rem;
  }
  .heading {
    display: flex;
    justify-content: space-between;
    align-items: start;
    gap: 1rem;
  }
  .heading h2,
  .heading p {
    margin: 0.2rem 0;
  }
  .heading p,
  .entries span {
    color: var(--secondary-text);
  }
  button {
    font: inherit;
    color: var(--text);
    background: var(--surface-raised, var(--surface));
    border: 1px solid var(--stroke);
    border-radius: 10px;
    padding: 0.65rem 0.8rem;
    cursor: pointer;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(16rem, 0.8fr) minmax(0, 1.5fr);
    gap: 1rem;
    align-items: start;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.5rem;
    max-height: 70vh;
    overflow: auto;
  }
  .entries button {
    width: 100%;
    display: grid;
    gap: 0.25rem;
    text-align: left;
  }
  .entries button.selected {
    outline: 2px solid var(--accent);
  }
  .date {
    font-size: 0.85em;
  }
  .detail {
    min-width: 0;
    padding: 1rem;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--surface);
  }
  .detail header {
    margin-bottom: 1rem;
  }
  .detail h3 {
    margin-top: 0;
  }
  .detail p {
    overflow-wrap: anywhere;
  }
  .error {
    color: var(--danger, var(--text));
  }
  .empty {
    padding: 2rem;
    border: 1px dashed var(--stroke);
    border-radius: 14px;
  }
  @media (max-width: 800px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .entries {
      max-height: 16rem;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    * {
      scroll-behavior: auto !important;
      transition: none !important;
    }
  }
</style>
