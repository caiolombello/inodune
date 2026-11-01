<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { X } from '@lucide/svelte';
  let {
    title,
    onclose,
    children,
    wide = false,
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
    wide?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
    return () => dialog.close();
  });
</script>

<dialog bind:this={dialog} class:wide {onclose} aria-label={title}>
  <div class="dialog-head">
    <h2>{title}</h2>
    <button
      class="icon-button"
      aria-label="Fechar diálogo"
      onclick={() => dialog.close()}><X size={20} /></button
    >
  </div>
  <div class="dialog-body">{@render children()}</div>
</dialog>
