import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  base: './',
  plugins: [svelte()],
  worker: { format: 'es' },
  test: { include: ['src/**/*.test.ts', 'eslint-rules/**/*.test.ts'] },
});
