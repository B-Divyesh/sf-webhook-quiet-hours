import { defineConfig } from 'vitest/config';
import { resolve } from 'node:path';

export default defineConfig({
  root: resolve(__dirname),
  test: { environment: 'node', exclude: ['e2e/**', 'node_modules/**'] },
});
