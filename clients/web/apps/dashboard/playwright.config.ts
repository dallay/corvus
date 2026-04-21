import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  use: {
    baseURL: "http://127.0.0.1:4324",
    trace: "on-first-retry",
  },
  webServer: {
    command: "PLAYWRIGHT=true pnpm dev --host 127.0.0.1 --port 4324",
    url: "http://127.0.0.1:4324",
    reuseExistingServer: true,
    timeout: 60_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
