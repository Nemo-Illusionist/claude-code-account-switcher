import { describe, expect, it } from "vitest";
import type { Usage } from "./contract";
import {
  STALE_AFTER,
  currentPercent,
  formatAge,
  freshness,
  usageLabel,
} from "./freshness";
import { messages } from "./i18n";

const t = messages.en;

const usage = (source: Usage["source"], fetched_at: number | null): Usage => ({
  source,
  fetched_at,
  five_hour: { percent: 42, resets_at: 2_000 },
  seven_day: null,
});

describe("freshness", () => {
  it("ages a reading against the clock, not against when it was sent", () => {
    // Regression: the UI showed a fixed `age_seconds`, so "15m ago" never moved.
    const live = usage("live", 1_000);
    expect(freshness(live, 1_000)).toEqual({ state: "live", age: 0 });
    expect(freshness(live, 1_000 + STALE_AFTER - 1).state).toBe("live");
    expect(freshness(live, 1_000 + STALE_AFTER)).toEqual({
      state: "stale",
      age: 600,
    });
    expect(freshness(usage("cached", 1_000), 1_060)).toEqual({
      state: "cached",
      age: 60,
    });
  });

  it("never reports an age for a reading that does not exist", () => {
    expect(freshness(usage("unavailable", null), 5)).toEqual({
      state: "unavailable",
      age: null,
    });
    expect(usageLabel(t, usage("no_credentials", null), 5)).toBe(
      "No credentials available",
    );
  });

  it("treats a clock behind the reading as fresh rather than negative", () => {
    expect(freshness(usage("cached", 1_000), 900)).toEqual({
      state: "cached",
      age: 0,
    });
  });

  it("drops a window's figure from the second its reset passes", () => {
    const w = { percent: 42, resets_at: 2_000 };
    expect(currentPercent(w, 1_999)).toBe(42);
    expect(currentPercent(w, 2_000)).toBeNull();
    expect(currentPercent({ percent: 7, resets_at: null }, 9_999)).toBe(7);
    expect(currentPercent(null, 0)).toBeNull();
  });

  it("formats each age range at its boundaries", () => {
    expect([59, 60, 3599, 3600, 86399, 86400].map((s) => formatAge(t, s))).toEqual([
      "just now",
      "1m ago",
      "59m ago",
      "1h ago",
      "23h ago",
      "1d ago",
    ]);
  });
});
