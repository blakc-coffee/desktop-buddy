import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "tests/ui",
  testMatch: "*.spec.ts",
  timeout: 30_000,
  workers: 1,
  fullyParallel: false,
  outputDir: "artifacts/ui-test-results",
  reporter: [
    ["list"],
    ["html", { outputFolder: "artifacts/ui-report", open: "never" }],
  ],
  use: {
    baseURL: "http://127.0.0.1:1437",
    channel:
      process.env.BUDDY_BROWSER_CHANNEL ||
      (process.platform === "win32" ? "msedge" : undefined),
    headless: true,
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run dev -- --port 1437 --strictPort",
    url: "http://127.0.0.1:1437",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
