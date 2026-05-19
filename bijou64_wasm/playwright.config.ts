import { defineConfig, devices } from "@playwright/test";

/**
 * Cross-browser tests for the wasm-bodge-built `bijou64` npm package.
 *
 * The HTML page in `e2e/server/index.html` imports the auto-initialising
 * `dist/esm/web.js` entry point (base64-embedded wasm — no fetch needed),
 * exposes the module on `window.bijou64`, and sets `window.bijou64Ready`
 * once initialisation completes.
 *
 * `webServer` runs `http-server` directly out of the workspace so we can
 * import from `dist/`; the production package layout is exactly what
 * `wasm-bodge` writes there.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["html", { open: "never" }], ["list"]],
  use: {
    baseURL: "http://127.0.0.1:9892",
    trace: "on-first-retry",
    headless: true,
  },
  // WebKit is opt-in (set `PLAYWRIGHT_INCLUDE_WEBKIT=1`) because the
  // nixpkgs `playwright-driver.browsers` derivation does not ship a
  // webkit binary in the path layout that `@playwright/test` expects on
  // NixOS. CI runs on ubuntu-latest with `pnpm exec playwright install`,
  // which downloads its own browsers, so it sets the flag and gets full
  // cross-browser coverage.
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox",  use: { ...devices["Desktop Firefox"] } },
    ...(process.env.PLAYWRIGHT_INCLUDE_WEBKIT === "1"
      ? [{ name: "webkit", use: { ...devices["Desktop Safari"] } }]
      : []),
  ],
  webServer: {
    // Serve from the package directory so `./dist/...` resolves naturally.
    // Port 9892 to avoid colliding with subduction's e2e (9891) when both
    // dev shells run at once.
    command: "http-server -p 9892 -s -c-1 .",
    port: 9892,
    timeout: 60 * 1000,
    reuseExistingServer: !process.env.CI,
  },
});
