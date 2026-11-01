<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import { get } from 'svelte/store';
  import {
    createVirtualizer,
    defaultRangeExtractor,
  } from '@tanstack/svelte-virtual';
  import type { Candidate } from './model';
  import { t } from './i18n.svelte';
  let {
    items,
    children,
  }: { items: Candidate[]; children: Snippet<[Candidate]> } = $props();
  let scrollElement = $state<HTMLDivElement>();
  let paged = $state(false);
  let focusedIndex = $state<number | null>(null);
  let currentPage = $state(0);
  const pageSize = 100;
  const large = $derived(items.length > 100);
  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => null,
    estimateSize: () => 90,
    overscan: 6,
  });
  $effect(() => {
    const list = items;
    const element = scrollElement;
    const enabled = large && !paged;
    const focused = focusedIndex;
    untrack(() =>
      get(virtualizer).setOptions({
        count: list.length,
        getScrollElement: () => element ?? null,
        getItemKey: (index) => list[index].id,
        enabled,
        rangeExtractor: (range) =>
          [
            ...new Set([
              ...defaultRangeExtractor(range),
              ...(focused !== null && focused < list.length ? [focused] : []),
            ]),
          ].sort((a, b) => a - b),
      }),
    );
  });
  $effect(() => {
    const list = items;
    untrack(() => {
      currentPage = 0;
      focusedIndex = null;
      if (list.length) scrollElement?.scrollTo({ top: 0 });
    });
  });
  function measure(node: HTMLDivElement) {
    get(virtualizer).measureElement(node);
    return { destroy: () => get(virtualizer).measureElement(null) };
  }
</script>

{#if large}<div class="list-mode">
    <span
      >{items.length}
      {t('itens')} · {paged
        ? t('paginação acessível')
        : t('lista virtualizada')}</span
    ><button class="text-button" onclick={() => (paged = !paged)}
      >{paged ? t('Usar rolagem virtual') : t('Usar paginação')}</button
    >
  </div>{/if}
{#if large && !paged}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (A scrollable region needs keyboard focus for native scrolling keys.) -->
  <div
    onfocusin={(event) => {
      const element =
        event.target instanceof Element
          ? event.target.closest<HTMLElement>('[data-index]')
          : null;
      focusedIndex = element ? Number(element.dataset.index) : null;
    }}
    bind:this={scrollElement}
    class="virtual-scroll"
    role="region"
    aria-label={t('Lista virtualizada de itens; paginação disponível acima')}
    tabindex="0"
  >
    <div
      class="virtual-canvas"
      role="list"
      style:height={$virtualizer.getTotalSize() + 'px'}
    >
      {#each $virtualizer.getVirtualItems() as row (row.key)}<div
          class="virtual-row"
          role="listitem"
          aria-setsize={items.length}
          aria-posinset={row.index + 1}
          data-index={row.index}
          style:transform={'translateY(' + row.start + 'px)'}
          use:measure
        >
          {@render children(items[row.index])}
        </div>{/each}
    </div>
  </div>
{:else}
  <div role="list">
    {#each paged ? items.slice(currentPage * pageSize, (currentPage + 1) * pageSize) : items as item (item.id)}<div
        role="listitem"
      >
        {@render children(item)}
      </div>{/each}
  </div>
  {#if large}<nav class="list-pager" aria-label={t('Páginas da lista')}>
      <button
        class="button"
        disabled={currentPage === 0}
        onclick={() => currentPage--}>{t('Anterior')}</button
      ><span role="status"
        >{t('Página')}
        {currentPage + 1}
        {t('de')}
        {Math.ceil(items.length / pageSize)}</span
      ><button
        class="button"
        disabled={(currentPage + 1) * pageSize >= items.length}
        onclick={() => currentPage++}>{t('Próxima')}</button
      >
    </nav>{/if}
{/if}

<style>
  .list-mode,
  .list-pager {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 12px;
    color: var(--secondary-text);
    padding-block: 8px;
  }
  .virtual-scroll {
    height: min(65dvh, 640px);
    min-height: 300px;
    overflow: auto;
    overflow-anchor: none;
    contain: strict;
  }
  .virtual-canvas {
    position: relative;
    width: 100%;
  }
  .virtual-row {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
  }
  .list-pager {
    flex-direction: row;
    overflow: visible;
  }
  .list-pager button {
    width: auto;
    color: var(--text);
    border: 1px solid var(--stroke);
  }
</style>
