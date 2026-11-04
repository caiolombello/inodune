<script lang="ts">
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    Search,
    LockKeyhole,
    CircleAlert,
    CircleCheck,
    RefreshCw,
    Package,
  } from '@lucide/svelte';
  import { isDesktop } from './backend';
  import { compareInstalledSize, formatBytes } from './model';
  import Modal from './Modal.svelte';
  import { formatDate, formatNumber, t } from './i18n.svelte';
  import {
    countLabel,
    sourceLabel,
    uninstallError,
    type ErrorInfo,
  } from './present';

  type Item = {
    source: string;
    id: string;
    name: string;
    version: string | null;
    size_bytes: number | null;
    install_date: number | null;
    protected: boolean;
    notes: string[];
    signals: { kind: string; timestamp: number; confidence: string }[];
  };
  type Report = {
    source: string;
    status: string;
    warnings: string[];
    items: Item[];
  };
  type Detail = {
    id: string;
    name: string;
    version: string;
    arch?: string | null;
    size_bytes?: number | null;
    requested: boolean;
  };
  type UninstallPlan = {
    id: string;
    items: { id: string; version: string }[];
    expires_in_seconds: number;
    steam_handoff: boolean;
    details?: Detail[];
  };

  let sources = $state<Report[] | null>(null);
  let loading = $state(false);
  let loadError = $state('');
  let uninstallPlans = $state<UninstallPlan[]>([]);
  let picked = $state<string[]>([]);
  let uninstallBusy = $state(false);
  let preparing = $state(false);
  let result = $state<
    { kind: 'ok'; text: string } | { kind: 'error'; info: ErrorInfo } | null
  >(null);
  let resultBox: HTMLDivElement | undefined = $state();

  let query = $state('');
  let sourceFilter = $state('all');
  let stateFilter = $state('all');
  let sizeFilter = $state('any');
  let sort = $state('size');

  const all = $derived((sources ?? []).flatMap((source) => source.items));
  const key = (item: Item) => `${item.source}\u0000${item.id}`;
  const visible = $derived(
    all
      .filter((item) => {
        const text =
          `${item.name} ${item.id} ${item.version ?? ''}`.toLocaleLowerCase(
            'pt-BR',
          );
        if (query && !text.includes(query.trim().toLocaleLowerCase('pt-BR')))
          return false;
        if (sourceFilter !== 'all' && item.source !== sourceFilter)
          return false;
        if (stateFilter === 'removable' && item.protected) return false;
        if (stateFilter === 'protected' && !item.protected) return false;
        const size = item.size_bytes;
        if (sizeFilter === 'large' && (size === null || size < 100 * 1024 ** 2))
          return false;
        if (sizeFilter === 'huge' && (size === null || size < 1024 ** 3))
          return false;
        if (sizeFilter === 'unknown' && size !== null) return false;
        return true;
      })
      .sort((a, b) =>
        sort === 'name'
          ? a.name.localeCompare(b.name, 'pt-BR')
          : compareInstalledSize(a.size_bytes, b.size_bytes) ||
            a.name.localeCompare(b.name, 'pt-BR'),
      ),
  );
  const pickedItems = $derived(
    all.filter((item) => picked.includes(key(item))),
  );
  const pickedBytes = $derived(
    pickedItems.reduce((n, item) => n + (item.size_bytes ?? 0), 0),
  );
  const presentSources = $derived([
    ...new Set((sources ?? []).map((s) => s.source)),
  ]);
  const filtersActive = $derived(
    query !== '' ||
      sourceFilter !== 'all' ||
      stateFilter !== 'all' ||
      sizeFilter !== 'any',
  );

  // Plan details come from the backend view; the fallback only splits ids.
  const planDetails = $derived(
    uninstallPlans.flatMap((plan): Detail[] =>
      plan.details?.length
        ? plan.details
        : plan.items.map((item) => {
            const fields = item.id.split(';');
            return {
              id: item.id,
              name: fields.length === 4 ? fields[0] : item.id,
              version: item.version,
              arch: fields.length === 4 ? fields[2] : null,
              size_bytes: null,
              requested: true,
            };
          }),
    ),
  );
  const requestedDetails = $derived(planDetails.filter((d) => d.requested));
  const dependencyDetails = $derived(planDetails.filter((d) => !d.requested));
  const planKnownBytes = $derived(
    planDetails.reduce((n, d) => n + (d.size_bytes ?? 0), 0),
  );
  const planUnknown = $derived(
    planDetails.filter((d) => d.size_bytes == null).length,
  );
  const steamHandoff = $derived(uninstallPlans.some((p) => p.steam_handoff));

  function protectionText(item: Item): string {
    return item.notes[0] ?? 'Protegido pela política de segurança do sistema.';
  }
  function togglePick(item: Item) {
    const k = key(item);
    picked = picked.includes(k)
      ? picked.filter((id) => id !== k)
      : [...picked, k];
  }
  function clearFilters() {
    query = '';
    sourceFilter = 'all';
    stateFilter = 'all';
    sizeFilter = 'any';
  }
  async function showResult(next: typeof result) {
    result = next;
    await tick();
    resultBox?.scrollIntoView({ block: 'nearest' });
    resultBox?.focus();
  }
  async function prepareUninstall(items: Item[]) {
    const chosen = items.filter((item) => !item.protected);
    if (!chosen.length) return;
    preparing = true;
    result = null;
    uninstallPlans = [];
    try {
      const groups: { source: string; group: Item[] }[] = [];
      for (const item of chosen) {
        const existing = groups.find((entry) => entry.source === item.source);
        if (existing) existing.group.push(item);
        else groups.push({ source: item.source, group: [item] });
      }
      const prepared: UninstallPlan[] = [];
      for (const { source, group } of groups) {
        prepared.push(
          await invoke<UninstallPlan>('uninstall_prepare', {
            source,
            id: group[0].id,
            ids: group.map((item) => item.id),
          }),
        );
      }
      uninstallPlans = prepared;
    } catch (e) {
      await showResult({ kind: 'error', info: uninstallError(String(e)) });
    } finally {
      preparing = false;
    }
  }
  async function executeUninstall() {
    const plans = uninstallPlans;
    if (!plans.length) return;
    uninstallPlans = [];
    uninstallBusy = true;
    result = null;
    try {
      const notes: string[] = [];
      for (const plan of plans) {
        notes.push(
          await invoke<string>('uninstall_execute', { planId: plan.id }),
        );
      }
      picked = [];
      await load();
      await showResult({
        kind: 'ok',
        text: notes.map((note) => t(note)).join(' '),
      });
    } catch (e) {
      await showResult({ kind: 'error', info: uninstallError(String(e)) });
    } finally {
      uninstallBusy = false;
    }
  }
  async function load() {
    loading = true;
    loadError = '';
    try {
      sources = (await invoke<{ sources: Report[] }>('list_installed_apps'))
        .sources;
      const keys = new Set(
        sources.flatMap((s) => s.items.filter((i) => !i.protected).map(key)),
      );
      picked = picked.filter((k) => keys.has(k));
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }
  const statusLabel: Record<string, string> = {
    ok: 'consultado',
    partial: 'parcial',
    unavailable: 'indisponível',
    error: 'erro',
  };
</script>

<section class="apps" aria-label={t('Inventário de programas e jogos')}>
  <div class="apps-intro">
    <p>
      {t(
        'Escolha os programas e confirme uma vez. O plano mostra o que sai, inclusive dependências. Indícios de atividade não comprovam desuso.',
      )}
    </p>
    <button
      class="button primary"
      disabled={loading || uninstallBusy || !isDesktop}
      onclick={load}
      ><RefreshCw size={16} />
      {t(
        loading
          ? 'Consultando fontes…'
          : sources
            ? 'Atualizar lista'
            : 'Listar programas e jogos',
      )}</button
    >
  </div>

  {#if !isDesktop}<div class="notice" role="status">
      <CircleAlert size={20} />
      <p>
        {t(
          'Disponível no aplicativo desktop. Nenhum inventário real é carregado nesta visualização web.',
        )}
      </p>
    </div>{/if}
  {#if loadError}<div class="notice error" role="alert">
      <CircleAlert size={20} />
      <div>
        <strong>{t('Não foi possível listar os programas')}</strong>
        <p>{t(loadError)}</p>
      </div>
    </div>{/if}

  <div bind:this={resultBox} tabindex="-1" class="result-slot">
    {#if result?.kind === 'error' && result.info.cancelled}<div
        class="notice neutral"
        role="status"
      >
        <CircleAlert size={20} />
        <div>
          <strong>{result.info.title}</strong>
          <p>{result.info.detail}</p>
        </div>
      </div>{:else if result?.kind === 'error'}<div
        class="notice error"
        role="alert"
      >
        <CircleAlert size={20} />
        <div>
          <strong>{result.info.title}</strong>
          <p>{result.info.detail}</p>
        </div>
      </div>{:else if result?.kind === 'ok'}<div
        class="notice success"
        role="status"
      >
        <CircleCheck size={20} />
        <p>{result.text}</p>
      </div>{/if}
    {#if uninstallBusy}<div class="notice" role="status">
        <RefreshCw size={20} />
        <p>
          {t(
            'Aguardando o gerenciador de pacotes. Responda à janela do sistema, se aparecer; não repita a operação.',
          )}
        </p>
      </div>{/if}
  </div>

  {#if loading && !sources}<div
      class="skeleton-group"
      aria-label={t('Consultando inventário local…')}
    >
      <div class="skeleton"></div>
      <div class="skeleton"></div>
      <div class="skeleton"></div>
    </div>{:else if sources}
    <ul class="source-status" aria-label={t('Fontes consultadas')}>
      {#each sources as source (source.source)}<li
          class:muted={source.status === 'unavailable'}
          class:warn={source.status === 'partial' || source.status === 'error'}
        >
          <strong>{sourceLabel(source.source)}</strong>
          <span
            >{countLabel(source.items.length, 'item', 'itens')} · {t(
              statusLabel[source.status] ?? source.status,
            )}</span
          >
          {#each source.warnings as warning, index (index)}<small
              >{t(warning)}</small
            >{/each}
        </li>{/each}
    </ul>

    <div class="apps-toolbar">
      <label class="search"
        ><Search size={17} /><input
          aria-label={t('Buscar programa')}
          placeholder={t('Buscar por nome, pacote ou versão…')}
          bind:value={query}
        /></label
      >
      <select aria-label={t('Filtrar por fonte')} bind:value={sourceFilter}>
        <option value="all">{t('Todas as fontes')}</option>
        {#each presentSources as source (source)}<option value={source}
            >{sourceLabel(source)}</option
          >{/each}
      </select>
      <select aria-label={t('Filtrar por proteção')} bind:value={stateFilter}>
        <option value="all">{t('Qualquer estado')}</option>
        <option value="removable">{t('Somente removíveis')}</option>
        <option value="protected">{t('Somente protegidos')}</option>
      </select>
      <select aria-label={t('Filtrar por tamanho')} bind:value={sizeFilter}>
        <option value="any">{t('Qualquer tamanho')}</option>
        <option value="large">{t('100 MiB ou mais')}</option>
        <option value="huge">{t('1 GiB ou mais')}</option>
        <option value="unknown">{t('Tamanho desconhecido')}</option>
      </select>
      <select aria-label={t('Ordenar programas')} bind:value={sort}>
        <option value="size">{t('Maior tamanho')}</option>
        <option value="name">{t('Nome')}</option>
      </select>
    </div>
    <div class="apps-count" role="status" aria-live="polite">
      <span
        >{`${formatNumber(visible.length)} ${t('de')} ${countLabel(all.length, 'programa', 'programas')}`}</span
      >
      {#if filtersActive}<button class="text-button" onclick={clearFilters}
          >{t('Limpar filtros')}</button
        >{/if}
    </div>

    {#if visible.length}<ul class="apps-list">
        {#each visible as item (key(item))}
          {@const isPicked = picked.includes(key(item))}
          <li class="app-row" class:selected={isPicked}>
            <div class="app-check">
              {#if item.protected}<span class="lock" aria-hidden="true"
                  ><LockKeyhole size={16} /></span
                >{:else}<input
                  type="checkbox"
                  aria-label={`${t('Selecionar')} ${item.name}`}
                  checked={isPicked}
                  disabled={uninstallBusy || preparing}
                  onchange={() => togglePick(item)}
                />{/if}
            </div>
            <div class="app-main">
              <strong>{item.name}</strong>
              <span class="app-meta"
                >{item.version ??
                  t('Versão desconhecida')}{#if item.id !== item.name}
                  · <code>{item.id}</code>{/if}</span
              >
              {#if item.protected}<span class="app-protected"
                  >{t(protectionText(item))}</span
                >{/if}
              {#each item.signals as signal, index (index)}<span
                  class="app-signal"
                  >{`${t(
                    signal.kind === 'steam_last_played'
                      ? 'Último uso registrado pelo Steam'
                      : 'Alteração da pasta de dados Flatpak',
                  )}: ${formatDate(signal.timestamp * 1000, {
                    dateStyle: 'medium',
                  })} · ${t('confiança')} ${t(signal.confidence === 'low' ? 'baixa' : 'média')}`}</span
                >{/each}
            </div>
            <span class="badge neutral app-source"
              >{sourceLabel(item.source)}</span
            >
            <span class="app-size"
              >{#if item.size_bytes === null}<span aria-hidden="true">—</span
                ><span class="sr-only">{t('Tamanho desconhecido')}</span
                >{:else}{formatBytes(item.size_bytes)}{/if}</span
            >
            <div class="app-action">
              {#if item.protected}<span class="badge protected"
                  >{t('Protegido')}</span
                >{:else}<button
                  class="button"
                  aria-label={`${t(item.source === 'steam' ? 'Abrir no Steam' : 'Desinstalar')} ${item.name}`}
                  disabled={!isDesktop || loading || uninstallBusy || preparing}
                  onclick={() => prepareUninstall([item])}
                  >{t(
                    item.source === 'steam' ? 'Abrir no Steam' : 'Desinstalar',
                  )}</button
                >{/if}
            </div>
            {#if item.notes.length > (item.protected ? 1 : 0)}<details
                class="app-notes"
              >
                <summary>{t('Notas')}</summary>
                {#each item.notes.slice(item.protected ? 1 : 0) as note, index (index)}<p
                  >
                    {t(note)}
                  </p>{/each}
              </details>{/if}
          </li>
        {/each}
      </ul>{:else}<div class="empty-state">
        <Package size={28} />
        <h2>
          {t(
            all.length
              ? 'Nenhum programa com esses filtros'
              : 'Nenhum programa encontrado',
          )}
        </h2>
        <p>
          {t(
            all.length
              ? 'Ajuste a busca ou limpe os filtros.'
              : 'As fontes consultadas não retornaram programas.',
          )}
        </p>
        {#if filtersActive}<button class="button" onclick={clearFilters}
            >{t('Limpar filtros')}</button
          >{/if}
      </div>{/if}
  {:else if isDesktop && !loading}<div class="empty-state">
      <Package size={28} />
      <h2>{t('Veja o que está instalado')}</h2>
      <p>
        {t(
          'A consulta é somente leitura: APT/RPM, Flatpak, Snap e Steam. Nada é removido sem um plano confirmado.',
        )}
      </p>
    </div>{/if}

  {#if picked.length}<div class="selection-bar apps-selection">
      <div>
        <strong
          >{countLabel(
            picked.length,
            'programa selecionado',
            'programas selecionados',
          )}</strong
        ><span>{formatBytes(pickedBytes)} {t('instalados')}</span>
      </div>
      <div class="inline-actions">
        <button
          class="button"
          onclick={() => (picked = [])}
          disabled={uninstallBusy}>{t('Desmarcar todos')}</button
        ><button
          class="button primary"
          disabled={uninstallBusy || preparing || !isDesktop}
          onclick={() => prepareUninstall(pickedItems)}
          >{preparing
            ? t('Simulando remoção…')
            : `${t('Desinstalar')} ${countLabel(picked.length, 'programa', 'programas')}`}</button
        >
      </div>
    </div>{/if}
</section>

{#if uninstallPlans.length}
  <Modal
    title={t(
      steamHandoff
        ? 'Abrir desinstalação no Steam?'
        : 'Confirmar desinstalação',
    )}
    onclose={() => (uninstallPlans = [])}
  >
    <div class="uninstall-plan">
      {#if steamHandoff}<p>
          {t(
            'O Steam mostra a própria confirmação e faz a remoção. O Inodune não confirma o resultado.',
          )}
        </p>{:else}<p class="plan-total">
          <strong
            >{countLabel(requestedDetails.length, 'programa', 'programas') +
              (dependencyDetails.length
                ? ` + ${countLabel(dependencyDetails.length, 'dependência', 'dependências')}`
                : '')}</strong
          >
          <span
            >{countLabel(planDetails.length, 'pacote', 'pacotes')} · {formatBytes(
              planKnownBytes,
            )}
            {t('instalados')}{#if planUnknown}
              · {formatNumber(planUnknown)}
              {t('sem tamanho conhecido')}{/if}</span
          >
        </p>{/if}
      <h3>{t('Selecionados')}</h3>
      <ul class="plan-list">
        {#each requestedDetails as d (d.id)}<li>
            <strong>{d.name}</strong><span>{d.version}</span><span
              >{d.size_bytes == null ? '—' : formatBytes(d.size_bytes)}</span
            >
          </li>{/each}
      </ul>
      {#if dependencyDetails.length}<h3>
          {t('Dependências que também serão removidas')}
        </h3>
        <ul class="plan-list">
          {#each dependencyDetails as d (d.id)}<li>
              <strong>{d.name}</strong><span>{d.version}</span><span
                >{d.size_bytes == null ? '—' : formatBytes(d.size_bytes)}</span
              >
            </li>{/each}
        </ul>{/if}
      {#if !steamHandoff}<div class="notice" role="note">
          <CircleAlert size={20} />
          <p>
            {t(
              'Sem autoremove nem purge. O plano vale por 2 minutos e é conferido de novo antes de executar. O sistema pode pedir sua senha; o Inodune não a recebe.',
            )}
          </p>
        </div>{/if}
      <details>
        <summary>{t('Identificadores técnicos')}</summary>
        <ul class="plan-ids">
          {#each uninstallPlans.flatMap((plan) => plan.items) as removal (removal.id)}<li
            >
              <code>{removal.id}</code>
            </li>{/each}
        </ul>
      </details>
    </div>
    <div class="dialog-actions">
      <button class="button" onclick={() => (uninstallPlans = [])}
        >{t('Cancelar')}</button
      ><button
        class="button primary"
        disabled={uninstallBusy}
        onclick={executeUninstall}
        >{steamHandoff
          ? t('Abrir no Steam')
          : `${t('Desinstalar')} ${countLabel(planDetails.length, 'pacote', 'pacotes')}`}</button
      >
    </div>
  </Modal>
{/if}

<style>
  .apps {
    display: grid;
    gap: 16px;
  }
  .apps-intro {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .apps-intro p {
    margin: 0;
    max-width: 62ch;
    color: var(--secondary-text);
  }
  .result-slot:focus {
    outline: none;
  }
  .result-slot:empty {
    display: none;
  }
  .result-slot :global(.notice) {
    margin-bottom: 0;
  }
  .notice strong {
    display: block;
    margin-bottom: 2px;
  }
  .notice.neutral {
    background: var(--subtle);
    color: var(--text);
  }
  .notice.success {
    background: var(--safe-bg);
    color: var(--safe-text);
  }
  .source-status {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .source-status li {
    display: grid;
    gap: 2px;
    padding: 8px 12px;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface);
    font-size: 12px;
    min-width: 9rem;
    max-width: 22rem;
  }
  .source-status li span,
  .source-status li small {
    color: var(--secondary-text);
  }
  .source-status li.muted {
    border-style: dashed;
  }
  .source-status li.warn {
    border-color: var(--warning);
  }
  .apps-toolbar {
    display: grid;
    grid-template-columns: minmax(14rem, 2fr) repeat(4, minmax(9rem, 1fr));
    gap: 8px;
  }
  .apps-count {
    display: flex;
    gap: 12px;
    align-items: center;
    font-size: 13px;
    color: var(--secondary-text);
    min-height: 28px;
  }
  .apps-list {
    list-style: none;
    margin: 0;
    padding: 0;
    background: var(--surface);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    overflow: hidden;
  }
  .app-row {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) 9rem 6.5rem 9.5rem;
    gap: 12px;
    align-items: center;
    padding: 12px 16px;
  }
  .app-row + .app-row {
    border-top: 1px solid var(--stroke);
  }
  .app-row.selected {
    background: var(--selected);
  }
  .app-check input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .lock {
    color: var(--protected-text);
    display: inline-flex;
  }
  .app-main {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .app-main strong {
    overflow-wrap: anywhere;
  }
  .app-meta,
  .app-signal {
    font-size: 12px;
    color: var(--secondary-text);
    overflow-wrap: anywhere;
  }
  .app-protected {
    font-size: 12px;
    color: var(--protected-text);
  }
  .app-source {
    justify-self: start;
  }
  .app-size {
    text-align: right;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .app-action {
    justify-self: end;
  }
  .badge.protected {
    background: var(--protected-bg);
    color: var(--protected-text);
  }
  .app-notes {
    grid-column: 2 / -1;
    font-size: 12px;
    border: 0;
    padding: 0;
    margin: 0;
    background: none;
  }
  .app-notes summary {
    color: var(--secondary-text);
  }
  .apps-selection {
    position: sticky;
    bottom: 12px;
  }
  .uninstall-plan h3 {
    font-size: 13px;
    margin: 16px 0 6px;
  }
  .plan-total {
    display: grid;
    gap: 2px;
    margin: 0;
  }
  .plan-total strong {
    font-size: 18px;
  }
  .plan-total span {
    color: var(--secondary-text);
  }
  .plan-list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
  }
  .plan-list li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 5.5rem;
    gap: 12px;
    padding: 8px 12px;
    font-size: 13px;
  }
  .plan-list li + li {
    border-top: 1px solid var(--stroke);
  }
  .plan-list span {
    color: var(--secondary-text);
    font-variant-numeric: tabular-nums;
  }
  .plan-list span:last-child {
    text-align: right;
  }
  .plan-ids {
    list-style: none;
    padding: 0;
    font-size: 11px;
  }
  .plan-ids code {
    overflow-wrap: anywhere;
  }
  .uninstall-plan :global(.notice) {
    margin: 16px 0 8px;
  }
  @media (max-width: 960px) {
    .apps-toolbar {
      grid-template-columns: 1fr 1fr;
    }
    .apps-toolbar .search {
      grid-column: 1 / -1;
    }
    .app-row {
      grid-template-columns: 28px minmax(0, 1fr) auto;
    }
    .app-source {
      display: none;
    }
    .app-action {
      grid-column: 2 / -1;
      justify-self: start;
    }
  }
</style>
