<script lang="ts">
  import license from '../../../../assets/fonts/Inter-OFL.txt?raw';
  import { language } from './i18n.svelte';
  const en = $derived(language.value === 'en-US');
</script>

<details>
  <summary
    >{en ? 'Font credits and license' : 'Créditos e licença da fonte'}</summary
  >
  <p>
    {en
      ? 'Inter 4.1 by The Inter Project Authors, bundled locally under SIL OFL 1.1. No font downloads while using the app.'
      : 'Inter 4.1 por The Inter Project Authors, incluída localmente sob SIL OFL 1.1. Sem downloads de fontes durante o uso.'}
  </p>
  <pre>{license}</pre>
</details>

<style>
  details {
    margin-top: var(--space-6);
  }
  summary {
    cursor: pointer;
    color: var(--secondary-text);
    font-size: 14px;
  }
  p {
    margin-top: var(--space-3);
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 12px;
    color: var(--secondary-text);
    max-height: 320px;
    overflow-y: auto;
    padding: var(--space-4);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
  }
</style>
