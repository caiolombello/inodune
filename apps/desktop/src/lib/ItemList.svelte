<script lang="ts">
  import { Folder, ChevronRight, LockKeyhole } from '@lucide/svelte';
  import { formatBytes, isLocallyEligible, type Candidate } from './model';
  import RiskBadge from './RiskBadge.svelte';
  import { protectionReason } from './protection';
  import VirtualList from './VirtualList.svelte';
  import { t } from './i18n.svelte';
  let {
    items,
    selected,
    ontoggle,
    ondetail,
    localMode = false,
    disabled = false,
  }: {
    items: Candidate[];
    disabled?: boolean;
    selected: string[];
    ontoggle: (item: Candidate) => void;
    ondetail: (item: Candidate) => void;
    localMode?: boolean;
  } = $props();
  const isSelectable = (item: Candidate) =>
    localMode ? isLocallyEligible(item) : item.risk !== 'protected';
</script>

<div class="item-list">
  <div class="list-header">
    <span>{t('Item / origem')}</span><span>{t('Tamanho lógico')}</span><span
      >{t('Política')}</span
    ><span></span>
  </div>
  {#if items.length}<VirtualList {items}
      >{#snippet children(item)}
        <div class="item-row" class:selected={selected.includes(item.id)}>
          <div class="item-name">
            {#if item.risk === 'protected'}<span
                class="locked"
                aria-label={t('Item protegido')}><LockKeyhole size={17} /></span
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
              ><span class="path">{item.display_path}</span>
            </div>
          </div>
          <strong class="item-size"
            >{item.coverage_key
              ? formatBytes(item.logical_bytes)
              : t('Não somado')}</strong
          >
          <div>
            <RiskBadge risk={item.risk} />
            {#if item.risk === 'protected'}<small
                class="item-policy-note"
                title={item.reason}>{t(protectionReason(item.reason))}</small
              >{/if}
            {#if localMode && item.risk === 'review' && isSelectable(item)}<small
                class="item-policy-note">{t('Revisão individual')}</small
              >{:else if !isSelectable(item)}<small class="item-policy-note"
                >{t('Somente análise')}</small
              >{/if}
          </div>
          <button
            class="icon-button detail-trigger"
            aria-label={t('Por que apareceu:') + ' ' + t(item.title)}
            onclick={() => ondetail(item)}><ChevronRight size={17} /></button
          >
        </div>
      {/snippet}</VirtualList
    >{:else}<div class="empty-state">
      <Folder size={28} />
      <h3>{t('Nenhum item nesta seleção')}</h3>
      <p>{t('Ajuste os filtros ou escolha outro escopo de exemplo.')}</p>
    </div>{/if}
</div>
