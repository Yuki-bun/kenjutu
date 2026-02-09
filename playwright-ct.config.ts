import { defineConfig, devices } from "@playwright/experimental-ct-react"
import { fileURLToPath } from "url"
import { dirname, resolve } from "path"
import tailwindcss from "@tailwindcss/vite"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

export default defineConfig({
  testDir: "./src",
  testMatch: "**/*.pw.test.tsx",
  snapshotDir: "./__snapshots__",
  timeout: 10 * 1000,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    trace: "on-first-retry",
    ctPort: 3100,
    ctViteConfig: {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      plugins: [tailwindcss() as any],
      resolve: {
        alias: {
          "@": resolve(__dirname, "./src"),
        },
      },
    },
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
})
