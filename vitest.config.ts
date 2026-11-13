import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts", "scripts/**/*.test.mjs"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["**/*.test.ts"],
      reporter: ["json-summary", "text"],
      reportsDirectory: "coverage/frontend",
    },
  },
});
