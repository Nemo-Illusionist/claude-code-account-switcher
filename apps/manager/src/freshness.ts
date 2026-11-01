import type { Usage, UsageWindow } from "./contract";
import type { Messages } from "./i18n";

/** A reading older than this is stale, whichever source produced it. */
export const STALE_AFTER = 10 * 60;

export type Freshness = "live" | "cached" | "stale" | "unavailable";

/**
 * How trustworthy a usage reading is at `now` (Unix seconds). Pure, so the
 * clock is a parameter: the same reading moves from live to stale as time
 * passes, without a new snapshot.
 */
export function freshness(
  usage: Usage,
  now: number,
): { state: Freshness; age: number | null } {
  if (usage.fetched_at == null) return { state: "unavailable", age: null };
  const age = Math.max(0, now - usage.fetched_at);
  if (age >= STALE_AFTER) return { state: "stale", age };
  return { state: usage.source === "live" ? "live" : "cached", age };
}

/** The figure to show for a window, or null once its reset has passed. */
export function currentPercent(
  window: UsageWindow | null,
  now: number,
): number | null {
  if (!window) return null;
  return resetPassed(window, now) ? null : window.percent;
}

export function resetPassed(window: UsageWindow, now: number): boolean {
  return window.resets_at != null && window.resets_at <= now;
}

export function formatAge(t: Messages, seconds: number): string {
  if (seconds < 60) return t.ages.justNow;
  if (seconds < 3600) return t.ages.minutes(Math.floor(seconds / 60));
  if (seconds < 86400) return t.ages.hours(Math.floor(seconds / 3600));
  return t.ages.days(Math.floor(seconds / 86400));
}

export function usageLabel(t: Messages, usage: Usage, now: number): string {
  const { state, age } = freshness(usage, now);
  switch (state) {
    case "live":
      return t.usage.live(formatAge(t, age!));
    case "cached":
      return t.usage.cached(formatAge(t, age!));
    case "stale":
      return t.usage.stale(formatAge(t, age!));
    case "unavailable":
      return usage.source === "no_credentials"
        ? t.usage.noCredentials
        : t.usage.unavailable;
  }
}
