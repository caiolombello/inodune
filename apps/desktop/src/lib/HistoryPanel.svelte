<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import {
    CircleCheck,
    CircleAlert,
    CircleSlash,
    FileWarning,
    History as HistoryIcon,
  } from '@lucide/svelte';
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
    <button class="button" onclick={refresh} disabled={loading}
      >{loading ? t('Carregando…') : t('Atualizar')}</button
    >
  </div>
  {#if error}<p class="error" role="alert">{t('Erro')}: {error}</p>{/if}
  {#if !loading && entries.length === 0 && !error}<div class="empty">
      <HistoryIcon size={26} />
      <h3>{t('Nenhum relatório salvo')}</h3>
      <p>{t('Relatórios finalizados aparecerão aqui.')}</p>
    </div>{/if}
  {#if entries.length}<div class="layout">
      <ul class="entries">
        {#each entries as entry (entry.report_id)}<li>
            <button
              class="entry"
              class:selected={selected?.report_id === entry.report_id}
              aria-current={selected?.report_id === entry.report_id
                ? 'true'
                : undefined}
              data-state={entry.error
                ? 'error'
                : entry.cancelled
                  ? 'cancelled'
                  : entry.complete
                    ? 'complete'
                    : 'partial'}
              onclick={() => open(entry)}
            >
              <span class="state-icon" aria-hidden="true"
                >{#if entry.error}<FileWarning
                    size={18}
                  />{:else if entry.cancelled}<CircleSlash
                    size={18}
                  />{:else if entry.complete}<CircleCheck
                    size={18}
                  />{:else}<CircleAlert size={18} />{/if}</span
              >
              <span class="date"
                >{entry.created_unix
                  ? formatDate(entry.created_unix * 1000)
                  : t('Data indisponível')}</span
              >
              {#if entry.error}<strong>{t('Relatório com erro')}</strong
                >{:else}<strong
                  >{actionLabel(entry.action).replace(/^./, (c) =>
                    c.toUpperCase(),
                  )} · {entry.cancelled
                    ? t('Cancelado')
                    : entry.complete
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
            <!-- State read from the report itself, not from the index row. -->
            <p
              class="report-state"
              data-state={report.cancelled
                ? 'cancelled'
                : report.complete
                  ? 'complete'
                  : 'partial'}
            >
              {#if report.cancelled}<CircleSlash
                  size={16}
                />{:else if report.complete}<CircleCheck
                  size={16}
                />{:else}<CircleAlert size={16} />{/if}
              <strong
                >{actionLabel(report.action).replace(/^./, (c) =>
                  c.toUpperCase(),
                )} · {report.cancelled
                  ? t('Cancelado')
                  : report.complete
                    ? t('Concluído')
                    : t('Incompleto')}</strong
              >
            </p>
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
            <button
              class="button"
              onclick={exportSelected}
              disabled={exporting}
            >
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
          {@const omitted = report.items.reduce(
            (n, item) => n + (item.omitted_files ?? 0),
            0,
          )}
          {#if omitted > 0}<p class="omitted" role="note">
              {countLabel(
                omitted,
                'linha de arquivo omitida neste relatório salvo; os totais estão completos.',
                'linhas de arquivo omitidas neste relatório salvo; os totais estão completos.',
              )}
            </p>{/if}
          <FileOutcomes files={report.items.flatMap((item) => item.files)} />
        {:else if selected}<p>
            {reportLoading
              ? t('Carregando relatório…')
              : t('Relatório indisponível')}
          </p>{:else}<div class="detail-empty">
            <HistoryIcon size={24} />
            <p>
              {t(
                'Selecione um relatório para ver o que aconteceu com cada arquivo.',
              )}
            </p>
          </div>{/if}
      </div>
    </div>{/if}
</section>

<style>
  .history {
    display: grid;
    gap: var(--space-4);
  }
  .heading {
    display: flex;
    justify-content: space-between;
    align-items: start;
    gap: var(--space-4);
  }
  .heading h2 {
    margin: 0 0 2px;
  }
  .heading p {
    margin: 0;
    font-size: var(--text-sm);
  }
  .heading p,
  .entry span {
    color: var(--secondary-text);
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(16rem, 0.8fr) minmax(0, 1.5fr);
    gap: var(--space-4);
    align-items: start;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
    max-height: 70vh;
    overflow: auto;
  }
  .entry {
    width: 100%;
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    column-gap: 10px;
    row-gap: 2px;
    text-align: left;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-inner);
    padding: 12px 14px;
    cursor: pointer;
    transition: border-color var(--motion-instant) var(--ease-standard);
  }
  .entry > :not(.state-icon) {
    grid-column: 2;
  }
  .entry strong {
    font-size: var(--text-md);
    font-weight: 600;
  }
  .entry:hover {
    border-color: var(--accent);
  }
  .entry.selected {
    background: var(--selected);
    border-color: var(--accent);
  }
  .state-icon {
    grid-row: 1 / span 3;
    color: var(--muted);
    padding-top: 1px;
  }
  .entry[data-state='complete'] .state-icon {
    color: var(--safe-text);
  }
  .entry[data-state='partial'] .state-icon {
    color: var(--warning-text);
  }
  .entry[data-state='error'] .state-icon {
    color: var(--danger);
  }
  .date {
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }
  .detail {
    min-width: 0;
    padding: var(--space-5);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .detail header {
    margin-bottom: var(--space-4);
    display: grid;
    gap: var(--space-2);
    justify-items: start;
  }
  .detail h3 {
    margin: 0;
  }
  .detail p {
    overflow-wrap: anywhere;
    margin: 0;
  }
  .report-state {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .report-state strong {
    color: var(--text);
  }
  .report-state[data-state='complete'] {
    color: var(--safe-text);
  }
  .report-state[data-state='partial'] {
    color: var(--warning-text);
  }
  .detail-empty {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--secondary-text);
  }
  .detail-empty :global(svg) {
    color: var(--accent);
  }
  .error {
    color: var(--danger);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    text-align: center;
    padding: var(--space-8);
    border: 1px dashed var(--control-border);
    border-radius: var(--radius-card);
    color: var(--secondary-text);
  }
  .empty :global(svg) {
    color: var(--accent);
  }
  .empty h3 {
    margin: 0;
    color: var(--text);
  }
  @media (max-width: 800px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .entries {
      max-height: 16rem;
    }
  }
</style>
