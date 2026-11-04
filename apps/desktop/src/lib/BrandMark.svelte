<script lang="ts">
  // Inline copy of assets/brand/inodune-icon.svg whose colours follow the
  // theme tokens (the file's fixed #164F52 stroke almost vanishes on dark).
  let { size = 36 }: { size?: number } = $props();
</script>

<svg
  class="brand-mark"
  width={size}
  height={size}
  viewBox="0 0 256 256"
  aria-hidden="true"
  focusable="false"
>
  <g fill="none" stroke-linecap="round" stroke-width="20">
    <path d="M166 40 A87 87 0 1 0 182 183" stroke="var(--brand-arc-outer)" />
    <path d="M158 81 A52 52 0 1 0 160 161" stroke="var(--brand-arc-inner)" />
  </g>
  <rect
    x="184"
    y="42"
    width="32"
    height="32"
    rx="9"
    fill="var(--brand-block)"
    transform="rotate(10 200 58)"
  />
</svg>

<style>
  .brand-mark {
    flex: 0 0 auto;
    display: block;
  }
</style>
