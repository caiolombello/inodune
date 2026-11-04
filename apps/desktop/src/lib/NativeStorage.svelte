<script lang="ts">
  import {
    HardDrive,
    ArrowRight,
    ShieldCheck,
    CircleAlert,
  } from '@lucide/svelte';
  import { formatBytes, type Policy, type ScanState } from './model';
  import type { NativeVolume } from './backend';
  import { formatNumber, t } from './i18n.svelte';
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
    selectedLabel = '',
    policy = 'trash',
    automatic = false,
    partial = false,
    potentialUncertain = false,
    onprotected,
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
    /** Estimate of the selection, already formatted (or "unavailable"). */
    selectedLabel?: string;
    policy?: Policy;
    automatic?: boolean;
    /** The analysis skipped areas: say so next to the cleanup action. */
    partial?: boolean;
    /** Some candidates have no summable size: the total is a lower bound. */
    potentialUncertain?: boolean;
    onprotected?: () => void;
  } = $props();
  let device = $state(0);
  const unique = $derived([
    ...new Map(volumes.map((v) => [v.device, v])).values(),
  ]);
  const volume = $derived(unique[device] ?? unique[0]);
  const measured = $derived(
    !!volume && volume.total_bytes > 0 && scanStatus !== 'idle',
  );
  const usedBytes = $derived(
    volume ? Math.max(0, volume.total_bytes - volume.available_bytes) : 0,
  );
  const pct = (bytes: number) =>
    volume && volume.total_bytes
      ? formatNumber((bytes / volume.total_bytes) * 100, {
          maximumFractionDigits: bytes / volume.total_bytes < 0.01 ? 1 : 0,
        })
      : '0';
  const R = 70;
  const C = 2 * Math.PI * R;
  // Only measured values are drawn: the selection is a logical estimate and
  // is shown apart, never as a slice of the disk it may not free.
  const usedFrac = $derived(
    volume && volume.total_bytes ? usedBytes / volume.total_bytes : 0,
  );
  const showCta = $derived(scanStatus !== 'idle' && scanStatus !== 'error');
</script>

<section class="storage-panel" aria-label={t('Volumes analisados')}>
  <div class="volume-summary">
    {#if volume && measured}<div
        class="disk-ring"
        role="img"
        aria-label={`${t('Volume')}: ${formatBytes(volume.total_bytes)} ${t('totais')}, ${formatBytes(usedBytes)} ${t('ocupados')}, ${formatBytes(volume.available_bytes)} ${t('disponíveis')}`}
      >
        <svg viewBox="0 0 172 172" aria-hidden="true">
          <circle class="track" cx="86" cy="86" r={R} />
          <circle
            class="used"
            cx="86"
            cy="86"
            r={R}
            stroke-dasharray={`${usedFrac * C} ${C}`}
          />
        </svg>
        <div class="ring-center">
          <strong>{formatBytes(volume.available_bytes)}</strong>
          <small>{t('disponíveis')}</small>
          <small class="of">{t('de')} {formatBytes(volume.total_bytes)}</small>
        </div>
      </div>{:else}<div class="unmeasured-volume">
        <HardDrive size={36} /><span>{t('Sem medição')}</span>
      </div>{/if}
    <div class="storage-legend">
      <h2>
        {volume && measured
          ? t('Seu disco agora')
          : t('Seu disco ainda não foi analisado.')}
      </h2>
      {#if volume && measured}<dl class="legend-rows">
          <div>
            <dt><i class="legend-dot used"></i> {t('Ocupado')}</dt>
            <dd>
              <strong>{formatBytes(usedBytes)}</strong><span
                >{pct(usedBytes)}%</span
              >
            </dd>
          </div>
          <div>
            <dt><i class="legend-dot free"></i> {t('Disponível')}</dt>
            <dd>
              <strong>{formatBytes(volume.available_bytes)}</strong><span
                >{pct(volume.available_bytes)}%</span
              >
            </dd>
          </div>
        </dl>
        <small
          >{displayPath(volume.scope_path)} · {t('medido nesta análise')}</small
        >{:else}<p>
          {t(
            automatic
              ? 'A home e as unidades locais montadas são descobertas automaticamente. Áreas protegidas ficam fora da limpeza.'
              : 'Somente pastas que você aprovar entram na análise.',
          )}
        </p>
        <small
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
    <span class="eyebrow">{t('SELECIONADO PARA LIMPAR')}</span>
    <div class="big-number">
      {!showCta ? '—' : selectedCount ? selectedLabel : formatBytes(0)}
    </div>
    <p class="selection-line">
      {#if showCta}{countLabel(
          selectedCount,
          'item selecionado',
          'itens selecionados',
        )} · {t('de')}
        {potentialUncertain ? '≥ ' : ''}{formatBytes(potential)}
        {t('em')}
        {countLabel(count, 'candidato', 'candidatos')}{:else}{t(
          'A estimativa aparece depois da análise.',
        )}{/if}
    </p>
    {#if showCta}<p class="policy-note">
        {t(
          policy === 'trash'
            ? 'Estimativa lógica. Na Lixeira, os itens continuam ocupando espaço até ela ser esvaziada.'
            : 'Estimativa lógica. O espaço realmente liberado só aparece depois, medido no relatório.',
        )}
      </p>{/if}
    {#if showCta && partial}<p class="partial-note">
        <CircleAlert size={14} />
        {t('Análise parcial: algumas áreas ficaram fora. Veja abaixo.')}
      </p>{/if}
    {#if showCta}<button
        class="button primary"
        onclick={onclean}
        disabled={!count || !selectedCount || busy}
        >{cleanLabel} <ArrowRight size={17} /></button
      >{/if}
    <small class="clean-hint" aria-live="polite"
      ><ShieldCheck size={14} />
      {count && !selectedCount && scanStatus !== 'idle'
        ? t('Nenhum item marcado. Marque itens na lista abaixo.')
        : t('A limpeza só começa depois da confirmação.')}</small
    >
    {#if protectedCount && onprotected}<button
        class="text-button protected-link"
        onclick={onprotected}
        >{countLabel(protectedCount, 'item protegido', 'itens protegidos')}
        {t('fora da limpeza · ver motivos')}</button
      >{/if}
  </div>
</section>

<style>
  .disk-ring {
    position: relative;
    width: 172px;
    height: 172px;
    flex: 0 0 172px;
  }
  .disk-ring svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  .disk-ring circle {
    fill: none;
    stroke-width: 16;
  }
  .track {
    stroke: var(--chart-free);
  }
  .used {
    stroke: var(--chart-used);
  }
  .used {
    transition:
      stroke-dasharray var(--motion-emphasis) var(--ease-standard),
      stroke-dashoffset var(--motion-emphasis) var(--ease-standard);
  }
  .ring-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    line-height: 1.2;
  }
  .ring-center strong {
    font-size: var(--text-2xl);
    font-weight: 650;
    letter-spacing: -0.015em;
    font-variant-numeric: tabular-nums;
  }
  .ring-center small {
    font-size: var(--text-sm);
    color: var(--secondary-text);
  }
  .ring-center .of {
    margin-top: 2px;
    font-size: var(--text-xs);
    color: var(--muted);
  }
  .unmeasured-volume {
    width: 160px;
    height: 160px;
    flex-shrink: 0;
    border: 2px dashed var(--control-border);
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    font-size: var(--text-sm);
  }
  .legend-rows {
    margin: 4px 0 0;
  }
  .legend-rows > div {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding: 9px 0;
    border-bottom: 1px solid var(--stroke);
    font-size: var(--text-sm);
  }
  .legend-rows dt {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--secondary-text);
  }
  .legend-rows dd {
    margin: 0;
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-variant-numeric: tabular-nums;
  }
  .legend-rows dd strong {
    color: var(--text);
    font-weight: 600;
  }
  .legend-rows dd span {
    min-width: 5ch;
    text-align: right;
    color: var(--muted);
    font-size: var(--text-xs);
  }
  /* Two lines reserved: the count changes length as items are (un)checked,
     and the cleanup button below must not move. */
  .selection-line {
    min-height: 3em;
    line-height: 1.5;
  }
  .policy-note {
    margin: -10px 0 12px;
    font-size: var(--text-xs);
    color: var(--secondary-text);
    max-width: 46ch;
  }
  .partial-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 12px;
    font-size: var(--text-xs);
    color: var(--warning-text);
  }
  .storage-legend p {
    max-width: 42ch;
  }
  /* Fixed height: switching hints must not move the button. */
  .clean-hint {
    min-height: 2.8em;
    align-items: flex-start !important;
  }
  .protected-link {
    padding: 4px 0;
    min-height: 32px;
    font-size: var(--text-sm);
  }
  .storage-legend label {
    display: block;
    margin-top: 12px;
    font-size: var(--text-sm);
  }
  .storage-legend select {
    width: 100%;
  }
  @media (max-width: 1279px) {
    .disk-ring {
      width: 148px;
      height: 148px;
      flex-basis: 148px;
    }
  }
</style>
