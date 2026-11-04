<script lang="ts">
  import { Folder, ChevronDown, Info, LockKeyhole, Hand } from '@lucide/svelte';
  import {
    formatBytes,
    isCarefulItem,
    isLocallyEligible,
    type Candidate,
  } from './model';
  import RiskBadge from './RiskBadge.svelte';
  import VirtualList from './VirtualList.svelte';
  import { t } from './i18n.svelte';
  import { countLabel, displayPath, tidyReason } from './present';
  let {
    items,
    selected,
    ontoggle,
    onsetmany,
    ondetail,
    localMode = false,
    disabled = false,
  }: {
    items: Candidate[];
    disabled?: boolean;
    selected: string[];
    ontoggle: (item: Candidate) => void;
    onsetmany?: (ids: string[], checked: boolean) => void;
    ondetail: (item: Candidate) => void;
    localMode?: boolean;
  } = $props();
  const isSelectable = (item: Candidate) =>
    localMode ? isLocallyEligible(item) : item.risk !== 'protected';
  // Same detector and title (e.g. node_modules of several projects, in
  // different roots) become one row that expands to list each folder.
  // Protected items stay one per row, each with its own reason.
  const grouping = $derived.by(() => {
    // Local, non-reactive scratch map; the result is plain data.
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const members = new Map<string, Candidate[]>();
    const rows: Candidate[] = [];
    for (const item of items) {
      if (item.risk === 'protected') {
        rows.push(item);
        continue;
      }
      const key = `${item.detector_id}::${item.title}`;
      const list = members.get(key);
      if (list) list.push(item);
      else {
        members.set(key, [item]);
        rows.push(item);
      }
    }
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const byLead = new Map<string, Candidate[]>();
    for (const list of members.values())
      if (list.length > 1) byLead.set(list[0].id, list);
    return { rows, byLead };
  });
  let expanded = $state<string[]>([]);
  const toggleGroup = (id: string) =>
    (expanded = expanded.includes(id)
      ? expanded.filter((k) => k !== id)
      : [...expanded, id]);
  function setGroup(members: Candidate[], checked: boolean) {
    const ids = members.map((m) => m.id);
    if (onsetmany) onsetmany(ids, checked);
    else
      for (const m of members)
        if (selected.includes(m.id) !== checked) ontoggle(m);
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

<div class="item-list">
  <div class="list-header">
    <span>{t('Item / origem')}</span><span>{t('Tamanho lógico')}</span><span
      >{t('Risco')}</span
    ><span><span class="sr-only">{t('Detalhes')}</span></span>
  </div>
  {#if items.length}<VirtualList items={grouping.rows}
      >{#snippet children(item)}
        {@const members = grouping.byLead.get(item.id)}
        {#if members}
          {@const eligible = members.filter(isSelectable)}
          {@const on = eligible.filter((m) => selected.includes(m.id)).length}
          {@const open = expanded.includes(item.id)}
          <div class="item-row group-row" class:selected={on > 0}>
            <div class="item-name">
              <input
                type="checkbox"
                disabled={disabled || !eligible.length}
                aria-label={t('Selecionar') + ' ' + t(item.title)}
                checked={eligible.length > 0 && on === eligible.length}
                use:indeterminate={on > 0 && on < eligible.length}
                onchange={(event) =>
                  setGroup(eligible, event.currentTarget.checked)}
              />
              <Folder class="folder-icon" size={20} />
              <div>
                <button
                  class="item-title"
                  aria-expanded={open}
                  onclick={() => toggleGroup(item.id)}
                  >{t(item.title)}
                  <small>{countLabel(members.length, 'pasta', 'pastas')}</small
                  ></button
                ><span class="path"
                  >{displayPath(item.display_path)} {t('e outras')}</span
                >
              </div>
            </div>
            <strong class="item-size"
              >{members.every((m) => m.coverage_key)
                ? formatBytes(members.reduce((n, m) => n + m.logical_bytes, 0))
                : t('Não somado')}</strong
            >
            <div>
              <RiskBadge
                risk={members.some((m) => m.risk === 'review')
                  ? 'review'
                  : item.risk}
              />
              {#if members.some(isCarefulItem)}<small
                  class="item-policy-note careful"
                  ><Hand size={12} /> {t('Exige cuidado')}</small
                >{:else if !eligible.length}<small class="item-policy-note"
                  >{t('Somente análise')}</small
                >{/if}
            </div>
            <button
              class="icon-button detail-trigger"
              aria-expanded={open}
              aria-label={`${t(open ? 'Ocultar pastas de' : 'Mostrar pastas de')} ${t(item.title)}`}
              onclick={() => toggleGroup(item.id)}
              ><ChevronDown size={17} class={open ? 'flipped' : ''} /></button
            >
            {#if open}<ul
                class="group-paths"
                aria-label={`${t('Pastas de')} ${t(item.title)}`}
              >
                {#each members as m (m.id)}<li
                    class:selected={selected.includes(m.id)}
                  >
                    <input
                      type="checkbox"
                      disabled={disabled || !isSelectable(m)}
                      aria-label={t('Selecionar') +
                        ' ' +
                        displayPath(m.display_path)}
                      checked={selected.includes(m.id)}
                      onchange={() => ontoggle(m)}
                    />
                    <code>{displayPath(m.display_path)}</code>
                    <span
                      >{m.coverage_key
                        ? formatBytes(m.logical_bytes)
                        : t('Não somado')}</span
                    >
                    <button
                      class="icon-button"
                      aria-label={t('Por que apareceu:') +
                        ' ' +
                        displayPath(m.display_path)}
                      onclick={() => ondetail(m)}><Info size={16} /></button
                    >
                  </li>{/each}
              </ul>{/if}
          </div>
        {:else}
          <div class="item-row" class:selected={selected.includes(item.id)}>
            <div class="item-name">
              {#if item.risk === 'protected'}<span
                  class="locked"
                  role="img"
                  aria-label={t('Item protegido')}
                  ><LockKeyhole size={17} /></span
                >
              {:else}<input
                  type="checkbox"
                  disabled={disabled || !isSelectable(item)}
                  aria-label={t('Selecionar') + ' ' + t(item.title)}
                  checked={selected.includes(item.id)}
                  onchange={() => ontoggle(item)}
                />{/if}
              <Folder class="folder-icon" size={20} />
              <div>
                <button class="item-title" onclick={() => ondetail(item)}
                  >{t(item.title)}</button
                ><span class="path">{displayPath(item.display_path)}</span
                >{#if item.risk === 'protected'}<span class="protected-reason"
                    >{t(tidyReason(item.reason))}</span
                  >{/if}
              </div>
            </div>
            <strong class="item-size"
              >{item.coverage_key
                ? formatBytes(item.logical_bytes)
                : t('Não somado')}</strong
            >
            <div>
              <RiskBadge risk={item.risk} />
              {#if isCarefulItem(item)}<small class="item-policy-note careful"
                  ><Hand size={12} /> {t('Exige cuidado')}</small
                >{:else if item.risk !== 'protected' && !isSelectable(item)}<small
                  class="item-policy-note">{t('Somente análise')}</small
                >{/if}
            </div>
            <button
              class="icon-button detail-trigger"
              aria-label={t('Por que apareceu:') + ' ' + t(item.title)}
              title={t('Por que este item apareceu?')}
              onclick={() => ondetail(item)}><Info size={17} /></button
            >
          </div>
        {/if}
      {/snippet}</VirtualList
    >{:else}<div class="empty-state">
      <Folder size={28} />
      <h3>{t('Nenhum item nesta seleção')}</h3>
      <p>{t('Ajuste os filtros ou escolha outro escopo de exemplo.')}</p>
    </div>{/if}
</div>

<style>
  .item-title small {
    font-weight: 400;
    color: var(--secondary-text);
    margin-left: 6px;
  }
  .group-paths {
    grid-column: 1 / -1;
    list-style: none;
    margin: 0 0 0 30px;
    padding: 0;
    font-size: var(--text-sm);
  }
  .group-paths li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) 120px 36px;
    gap: 12px;
    align-items: center;
    min-height: 40px;
    border-radius: 8px;
    padding: 0 4px;
  }
  .group-paths li.selected {
    background: var(--selected);
  }
  .group-paths code {
    overflow-wrap: anywhere;
    color: var(--secondary-text);
  }
  .group-paths span {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .group-row :global(.flipped) {
    transform: rotate(180deg);
  }
  .group-row .detail-trigger :global(svg) {
    transition: transform var(--motion-fast) var(--ease-standard);
  }
</style>
