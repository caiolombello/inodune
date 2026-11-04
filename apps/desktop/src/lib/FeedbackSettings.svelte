<script lang="ts">
  import { Volume2, VolumeX, Music2 } from '@lucide/svelte';
  import { language } from './i18n.svelte';
  let {
    enabled,
    onchange,
    onpreview,
    message,
  }: {
    enabled: boolean;
    onchange: (enabled: boolean) => void;
    onpreview: () => void;
    message: string;
  } = $props();
  const en = $derived(language.value === 'en-US');
</script>

<div class="sound-settings">
  <label class="setting-row">
    <span>
      <strong class="sound-label">
        {#if enabled}<Volume2 size={18} />{:else}<VolumeX size={18} />{/if}
        {en ? 'Sound effects' : 'Efeitos sonoros'}
      </strong>
      <small
        >{en
          ? 'Quiet signals when analysis ends or an issue needs attention. Off by default. No background sounds.'
          : 'Sinais suaves ao encerrar a análise ou encontrar um problema. Desligados por padrão. Sem som em segundo plano.'}</small
      >
    </span>
    <input
      type="checkbox"
      class="switch"
      role="switch"
      checked={enabled}
      onchange={(event) => onchange(event.currentTarget.checked)}
    />
  </label>
  <button class="button" disabled={!enabled} onclick={onpreview}>
    <Music2 size={16} />
    {en ? 'Preview sound' : 'Ouvir exemplo'}
  </button>
  <p class="sound-note" role="status">{message}</p>
</div>

<style>
  .sound-label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .sound-note {
    min-height: 1.5em;
    font-size: 14px;
    color: var(--secondary-text);
  }
</style>
