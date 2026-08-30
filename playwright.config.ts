import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'frontend/e2e',
  timeout: 30_000,
  forbidOnly: Boolean(process.env.CI),
  use: {
    baseURL: 'http://127.0.0.1:4173',
    browserName: 'chromium',
    bypassCSP: true,
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'bash scripts/run-e2e-server.sh',
    url: 'http://127.0.0.1:4173/health',
    timeout: 120_000,
    reuseExistingServer: !process.env.CI,
  },
});
