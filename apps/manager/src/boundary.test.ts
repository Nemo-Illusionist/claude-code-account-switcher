import { expect, it } from "vitest";

// Every source file's text, as Vite sees it.
const sources = import.meta.glob(["./**/*.{ts,tsx}", "!./**/*.test.{ts,tsx}"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

it("only api.ts reaches the Tauri runtime", () => {
  // Requests and event subscriptions live in one module, so changing how the
  // backend is reached touches one file rather than every component.
  const importers = Object.entries(sources)
    .filter(([, text]) => /from\s+["']@tauri-apps\//.test(text))
    .map(([path]) => path);
  expect(importers).toStrictEqual(["./api.ts"]);
});
