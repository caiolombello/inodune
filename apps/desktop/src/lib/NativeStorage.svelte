<script lang="ts">
  import { HardDrive, ArrowRight, ShieldCheck } from '@lucide/svelte';
  import { formatBytes, type ScanState } from './model';
  import type { NativeVolume } from './backend';
  import { t } from './i18n.svelte';
  import { countLabel, displayPath } from './present';
  let {
    volumes,
    potential,
    count,
    protectedCount,
    busy,
    scanStatus,
    onclean,
    cleanLabel,
    selectedCount = 0,
    automatic = false,
  }: {
    volumes: NativeVolume[];
    potential: number;
    count: number;
    protectedCount: number;
    busy: boolean;
    scanStatus: ScanState;
    onclean: () => void;
    cleanLabel: string;
    selectedCount?: number;
    automatic?: boolean;
  } = $props();
  let device = $state(0);
  const unique = $derived([
    ...new Map(volumes.map((v) => [v.device, v])).values(),
  ]);
  const volume = $derived(unique[device] ?? unique[0]);
</script>

<section class="storage-panel" aria-label={t('Volumes analisados')}>
  <div class="volume-summary">
    {#if volume}<div
        class="ring"
        style:background={'conic-gradient(var(--teal) ' +
          360 * (1 - volume.available_bytes / volume.total_bytes) +
          'deg, var(--subtle) 0deg)'}
        role="img"
        aria-label={`${t('Volume')}: ${formatBytes(volume.total_bytes)} ${t('totais')}, ${formatBytes(volume.available_bytes)} ${t('disponíveis')}`}
      >
        <div>
          <HardDrive size={19} /><strong
            >{formatBytes(volume.total_bytes)}</strong
          ><small>{t('capacidade')}</small>
        </div>
      </div>{:else}<div class="unmeasured-volume">
        <HardDrive size={40} /><span>{t('Sem medição')}</span>
      </div>{/if}
    <div class="storage-legend">
      <h2>
        {volume
          ? t('Espaço medido, no seu disco.')
          : t('Seu disco ainda não foi analisado.')}
      </h2>
      <p>
        {t(
          automatic
            ? 'A home e as unidades locais montadas são descobertas automaticamente. Áreas protegidas ficam fora da limpeza.'
            : 'Somente pastas que você aprovar entram na análise.',
        )}
      </p>
      {#if volume}<div>
          <span
            ><i class="legend-dot used"></i> {t('Ocupado ou reservado')}</span
          ><strong
            >{formatBytes(volume.total_bytes - volume.available_bytes)}</strong
          >
        </div>
        <div>
          <span
            ><i class="legend-dot free"></i> {t('Disponível ao usuário')}</span
          ><strong>{formatBytes(volume.available_bytes)}</strong>
        </div>
        <small
          >{displayPath(volume.scope_path)} · {t('medido nesta análise')}</small
        >{:else}<small
          >{t(
            automatic
              ? 'Clique em Analisar computador. Não é necessário escolher pastas.'
              : 'Escolha uma pasta para começar. Nenhum número de demonstração é usado neste modo.',
          )}</small
        >{/if}
      {#if unique.length > 1}<label
          >{t('Volume')}
          <select aria-label={t('Volume')} bind:value={device}
            >{#each unique as vol, index (vol.device)}<option value={index}
                >{displayPath(vol.scope_path)}</option
              >{/each}</select
          ></label
        >{/if}
    </div>
  </div>
  <div class="potential-summary">
    <span class="eyebrow">{t('POTENCIAL LÓGICO ESTIMADO')}</span>
    <div class="big-number">
      {scanStatus === 'idle' || scanStatus === 'error'
        ? '—'
        : formatBytes(potential)}
    </div>
    <p>
      {countLabel(count, 'candidato', 'candidatos')} · {countLabel(
        protectedCount,
        'item protegido',
        'itens protegidos',
      )}
    </p>
    <button
      class="button primary"
      onclick={onclean}
      disabled={!count || !selectedCount || busy}
      >{cleanLabel} <ArrowRight size={17} /></button
    ><small class="clean-hint" aria-live="polite"
      ><ShieldCheck size={14} />
      {count && !selectedCount && scanStatus !== 'idle'
        ? t('Nenhum item marcado. Marque itens na lista abaixo.')
        : t('A limpeza só começa depois da confirmação.')}</small
    >
  </div>
</section>

<style>
  .unmeasured-volume {
    width: 160px;
    height: 160px;
    flex-shrink: 0;
    border: 2px dashed var(--stroke);
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
    justify-content: center;
    color: var(--muted);
  }
  .ring strong {
    font-size: 24px;
  }
  /* Fixed height: switching hints must not move the button. */
  .clean-hint {
    height: 3.4em;
    align-items: flex-start !important;
    overflow: hidden;
  }
  .storage-legend label {
    display: block;
    margin-top: 12px;
    font-size: 12px;
  }
  .storage-legend select {
    width: 100%;
  }
</style>
