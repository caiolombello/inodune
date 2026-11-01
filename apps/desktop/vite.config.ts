import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
export default defineConfig({
  plugins: [svelte()],
  server: { strictPort: true },
  test: { include: ['src/**/*.test.ts'] },
});
