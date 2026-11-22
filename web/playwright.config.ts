import { defineConfig, devices } from '@playwright/test';

const PORT = 4173;
const BASE = '/genetic-game-of-life/';
const url = `http://localhost:${PORT}${BASE}`;
const ci = Boolean(process.env.CI);

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  forbidOnly: ci,
  retries: ci ? 1 : 0,
  reporter: ci ? 'github' : 'list',
  use: { baseURL: url },
  projects: [
    {
      name: 'chromium',
      testIgnore: /mobile\.spec\.ts/,
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 900 },
        launchOptions: { args: ['--enable-unsafe-swiftshader'] },
      },
    },
    {
      name: 'mobile',
      testMatch: /mobile\.spec\.ts/,
      use: {
        ...devices['Pixel 7'],
        launchOptions: { args: ['--enable-unsafe-swiftshader'] },
      },
    },
  ],
  webServer: {
    command: `npm run preview -- --port ${PORT} --strictPort --base ${BASE}`,
    url,
    reuseExistingServer: !ci,
  },
});
