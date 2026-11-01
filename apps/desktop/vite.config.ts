import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
export default defineConfig({
  plugins: [svelte()],
  // Keep canonical SVGs as self-hosted assets; native CSP does not allow data: images.
  build: { assetsInlineLimit: 0 },
  server: { strictPort: true },
  test: { include: ['src/**/*.test.ts'] },
});
