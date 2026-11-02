import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

export default defineConfig(({ mode }) => {
  // Release (`vite build`) never ships the demo corpus. Dev, tests and
  // `INODUNE_DEMO=1` builds (Playwright) keep it.
  const demo = mode !== 'production' || process.env.INODUNE_DEMO === '1';
  const demoModule = demo ? './src/lib/demo-data.ts' : './src/lib/demo-empty.ts';
  return {
    plugins: [svelte()],
    define: { __INODUNE_DEMO__: JSON.stringify(demo) },
    resolve: {
      alias: {
        'virtual:inodune-demo': fileURLToPath(new URL(demoModule, import.meta.url)),
      },
    },
    // Keep canonical SVGs as self-hosted assets; native CSP does not allow data: images.
    build: { assetsInlineLimit: 0 },
    server: { strictPort: true },
    test: { include: ['src/**/*.test.ts'] },
  };
});
