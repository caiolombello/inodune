<script lang="ts">
  import { Folder, ChevronRight, LockKeyhole } from '@lucide/svelte';
  import { formatBytes, type Candidate } from './model';
  import RiskBadge from './RiskBadge.svelte';
  import VirtualList from './VirtualList.svelte';
  let {
    items,
    selected,
    ontoggle,
    ondetail,
    disabled = false,
  }: {
    items: Candidate[];
    disabled?: boolean;
    selected: string[];
    ontoggle: (item: Candidate) => void;
    ondetail: (item: Candidate) => void;
  } = $props();
</script>

<div class="item-list">
  <div class="list-header">
    <span>Item / origem</span><span>Tamanho lógico</span><span>Política</span
    ><span></span>
  </div>
  {#if items.length}<VirtualList {items}
      >{#snippet children(item)}
        <div class="item-row" class:selected={selected.includes(item.id)}>
          <div class="item-name">
            {#if item.risk === 'protected'}<span
                class="locked"
                aria-label="Item protegido"><LockKeyhole size={17} /></span
              >
            {:else}<input
                type="checkbox"
                {disabled}
                aria-label={'Selecionar ' + item.title}
                checked={selected.includes(item.id)}
                onchange={() => ontoggle(item)}
              />{/if}
            <Folder class="folder-icon" size={20} />
            <div>
              <button class="item-title" onclick={() => ondetail(item)}
                >{item.title}</button
              ><span class="path">{item.display_path}</span>
            </div>
          </div>
          <strong class="item-size"
            >{item.coverage_key
              ? formatBytes(item.logical_bytes)
              : 'Não somado'}</strong
          >
          <RiskBadge risk={item.risk} />
          <button
            class="icon-button detail-trigger"
            aria-label={'Por que apareceu: ' + item.title}
            onclick={() => ondetail(item)}><ChevronRight size={17} /></button
          >
        </div>
      {/snippet}</VirtualList
    >{:else}<div class="empty-state">
      <Folder size={28} />
      <h3>Nenhum item nesta seleção</h3>
      <p>Ajuste os filtros ou escolha outro escopo de exemplo.</p>
    </div>{/if}
</div>
