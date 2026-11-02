<script lang="ts">
  import { ChevronDown, ChevronRight, LockKeyhole } from '@lucide/svelte';
  import { formatBytes, isCarefulItem, type Candidate } from './model';
  import { formatNumber, t } from './i18n.svelte';
  import { countLabel, displayPath, tidyReason } from './present';

  let {
    candidates,
    selected,
    isSelectable,
    disabled = false,
    onsetmany,
    ondetail,
  }: {
    candidates: Candidate[];
    selected: string[];
    isSelectable: (item: Candidate) => boolean;
    disabled?: boolean;
    onsetmany: (ids: string[], checked: boolean) => void;
    ondetail: (item: Candidate) => void;
  } = $props();

  const picked = $derived(new Set(selected));
  // Grouped rows (e.g. several node_modules) expand to list each folder.
  let expanded = $state<string[]>([]);
  function toggleGroup(key: string) {
    expanded = expanded.includes(key)
      ? expanded.filter((k) => k !== key)
      : [...expanded, key];
  }
  type Group = { key: string; title: string; items: Candidate[] };
  // Repeated detections (40 × "Bytecode Python") collapse into one row.
  function groupsOf(items: Candidate[]): Group[] {
    // Local, non-reactive scratch map; the result is a plain array.
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const map = new Map<string, Group>();
    for (const item of items) {
      const key = `${item.detector_id}::${item.title}`;
      const group = map.get(key) ?? { key, title: item.title, items: [] };
      group.items.push(item);
      map.set(key, group);
    }
    return [...map.values()].sort((a, b) => bytes(b.items) - bytes(a.items));
  }
  const bytes = (items: Candidate[]) =>
    items.reduce((n, item) => n + item.logical_bytes, 0);
  const categories = $derived(
    [
      ...new Set(
        candidates.filter((c) => c.risk !== 'protected').map((c) => c.category),
      ),
    ].map((category) => {
      const items = candidates.filter(
        (c) => c.category === category && c.risk !== 'protected',
      );
      return { category, items, groups: groupsOf(items) };
    }),
  );
  const protectedItems = $derived(
    candidates.filter((c) => c.risk === 'protected'),
  );
  function groupState(items: Candidate[]) {
    const eligible = items.filter(isSelectable);
    const on = eligible.filter((item) => picked.has(item.id)).length;
    return {
      eligible,
      all: eligible.length > 0 && on === eligible.length,
      some: on > 0 && on < eligible.length,
      on,
    };
  }
  function indeterminate(node: HTMLInputElement, value: boolean) {
    node.indeterminate = value;
    return {
      update(next: boolean) {
        node.indeterminate = next;
      },
    };
  }
</script>

<div class="overview-review">
  {#each categories as cat (cat.category)}
    {@const catState = groupState(cat.items)}
    <section class="review-category">
      <header>
        <h3>{t(cat.category)}</h3>
        <span
          >{formatNumber(catState.on)}
          {t('de')}
          {countLabel(
            cat.items.length,
            'item selecionado',
            'itens selecionados',
          )}
          · {formatBytes(
            cat.items
              .filter((item) => picked.has(item.id))
              .reduce((n, item) => n + item.logical_bytes, 0),
          )}</span
        >
      </header>
      <ul>
        {#each cat.groups as group (group.key)}
          {@const g = groupState(group.items)}
          {@const first = group.items[0]}
          <li class="review-row" class:selected={g.on > 0}>
            <label>
              <input
                type="checkbox"
                checked={g.all}
                use:indeterminate={g.some}
                disabled={disabled || !g.eligible.length}
                onchange={(event) =>
                  onsetmany(
                    g.eligible.map((item) => item.id),
                    event.currentTarget.checked,
                  )}
              />
              <span class="row-title"
                >{t(group.title)}{#if group.items.length > 1}<small
                    >{countLabel(group.items.length, 'pasta', 'pastas')}</small
                  >{/if}</span
              >
            </label>
            <span class="row-path" title={displayPath(first.display_path)}
              >{group.items.length > 1
                ? `${displayPath(first.display_path)} ${t('e outras')}`
                : displayPath(first.display_path)}</span
            >
            <span class="row-size">{formatBytes(bytes(group.items))}</span>
            <span class="row-tags">
              {#if first.risk === 'review'}<span class="badge review"
                  >{t('Revisar')}</span
                >{:else if isCarefulItem(first)}<span class="badge neutral"
                  >{t('Exige cuidado')}</span
                >{:else if !g.eligible.length}<span class="badge neutral"
                  >{t('Somente análise')}</span
                >{/if}
            </span>
            {#if group.items.length === 1}<button
                class="icon-button"
                aria-label={t('Por que apareceu:') + ' ' + t(first.title)}
                onclick={() => ondetail(first)}
                ><ChevronRight size={16} /></button
              >{:else}<button
                class="icon-button"
                aria-expanded={expanded.includes(group.key)}
                aria-label={`${t(expanded.includes(group.key) ? 'Ocultar pastas de' : 'Mostrar pastas de')} ${t(group.title)}`}
                onclick={() => toggleGroup(group.key)}
                ><ChevronDown
                  size={16}
                  class={expanded.includes(group.key) ? 'flipped' : ''}
                /></button
              >{/if}
            {#if group.items.length > 1 && expanded.includes(group.key)}<ul
                class="group-paths"
                aria-label={`${t('Pastas de')} ${t(group.title)}`}
              >
                {#each group.items as item (item.id)}<li>
                    <code>{displayPath(item.display_path)}</code>
                    <span>{formatBytes(item.logical_bytes)}</span>
                    <button
                      class="icon-button"
                      aria-label={t('Por que apareceu:') +
                        ' ' +
                        displayPath(item.display_path)}
                      onclick={() => ondetail(item)}
                      ><ChevronRight size={14} /></button
                    >
                  </li>{/each}
              </ul>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
  {#if protectedItems.length}<section class="review-category protected">
      <header>
        <h3>{t('Protegidos')}</h3>
        <span
          >{countLabel(protectedItems.length, 'item', 'itens')} · {formatBytes(
            bytes(protectedItems),
          )} · {t('nunca entram na limpeza')}</span
        >
      </header>
      <ul>
        {#each protectedItems as item (item.id)}<li
            class="review-row protected-row"
          >
            <span class="lock" role="img" aria-label={t('Item protegido')}
              ><LockKeyhole size={15} /></span
            >
            <span class="row-title">{t(item.title)}</span>
            <span class="row-path" title={displayPath(item.display_path)}
              >{displayPath(item.display_path)}</span
            >
            <span class="row-size"
              >{item.coverage_key
                ? formatBytes(item.logical_bytes)
                : t('Não medido')}</span
            >
            <span class="row-reason">{t(tidyReason(item.reason))}</span>
          </li>{/each}
      </ul>
    </section>{/if}
</div>

<style>
  .overview-review {
    display: grid;
    gap: 18px;
    padding: 4px 0 8px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    flex-wrap: wrap;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--stroke);
  }
  h3 {
    margin: 0;
    font-size: 14px;
  }
  header span {
    font-size: 12px;
    color: var(--secondary-text);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .review-row {
    display: grid;
    grid-template-columns:
      minmax(12rem, 1.1fr) minmax(0, 1.6fr)
      6.5rem 7.5rem 32px;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 4px 8px;
    border-radius: 8px;
    font-size: 13px;
  }
  .review-row + .review-row {
    border-top: 1px solid color-mix(in srgb, var(--stroke) 60%, transparent);
  }
  .review-row.selected {
    background: var(--selected);
  }
  label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    min-width: 0;
  }
  input[type='checkbox'] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
    flex-shrink: 0;
  }
  .row-title {
    font-weight: 600;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .row-title small {
    font-weight: 400;
    color: var(--secondary-text);
  }
  .row-path {
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    color: var(--secondary-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .group-paths {
    grid-column: 1 / -1;
    list-style: none;
    margin: 0 0 4px 28px;
    padding: 0;
    font-size: 12px;
  }
  .group-paths li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 6.5rem 28px;
    gap: 12px;
    align-items: center;
  }
  .group-paths code {
    overflow-wrap: anywhere;
    color: var(--secondary-text);
  }
  .group-paths span {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .review-row :global(.flipped) {
    transform: rotate(180deg);
  }
  .row-size {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .protected-row {
    grid-template-columns:
      18px minmax(10rem, 0.9fr) minmax(0, 1.4fr)
      6.5rem minmax(0, 1.6fr);
  }
  .lock {
    color: var(--protected-text);
    display: inline-flex;
  }
  .row-reason {
    font-size: 12px;
    color: var(--secondary-text);
    line-height: 1.35;
  }
  @media (max-width: 900px) {
    .review-row,
    .protected-row {
      grid-template-columns: 1fr auto;
      row-gap: 2px;
    }
    .row-path,
    .row-reason {
      grid-column: 1 / -1;
      white-space: normal;
      overflow-wrap: anywhere;
    }
    .lock {
      display: none;
    }
    .row-tags:empty {
      display: none;
    }
  }
</style>
