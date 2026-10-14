import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: 'src/tests/e2e',
  fullyParallel: true,
  reporter: 'list',
  use: { ...devices['Desktop Chrome'], baseURL: 'http://127.0.0.1:1440', trace: 'on-first-retry' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 1440 --strictPort',
    // Keep release tests independent of the host's native file watcher limit.
    env: { CHOKIDAR_USEPOLLING: 'true' },
    url: 'http://127.0.0.1:1440',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
