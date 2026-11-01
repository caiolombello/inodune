<script lang="ts">
  import { ShieldCheck, Shield, CircleAlert } from '@lucide/svelte';
  import { riskLabel, type Risk } from './model';
  import { t } from './i18n.svelte';
  let { risk }: { risk: Risk } = $props();
</script>

<span
  class="badge {risk}"
  title={risk === 'protected'
    ? t(
        'Protegido é uma política de segurança. O inodune não detecta se o item está em uso.',
      )
    : undefined}
>
  {#if risk === 'low'}<ShieldCheck
      size={13}
    />{:else if risk === 'protected'}<Shield size={13} />{:else}<CircleAlert
      size={13}
    />{/if}
  {t(riskLabel[risk])}
</span>
