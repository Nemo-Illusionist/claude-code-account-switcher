import { useEffect, useState } from "react";

const seconds = () => Math.floor(Date.now() / 1000);

/** Unix seconds, re-read every `everyMs` so relative ages keep moving. */
export function useNow(everyMs = 30_000): number {
  const [now, setNow] = useState(seconds);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(seconds()), everyMs);
    return () => window.clearInterval(timer);
  }, [everyMs]);
  return now;
}

/** A per-viewer preference; storage failures fall back to `initial`. */
export function usePreference<T extends string | number>(
  key: string,
  initial: T,
  allowed: readonly T[],
): [T, (value: T) => void] {
  const [value, setValue] = useState<T>(() => {
    try {
      const stored = localStorage.getItem(key);
      return allowed.find((a) => String(a) === stored) ?? initial;
    } catch {
      return initial;
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem(key, String(value));
    } catch {
      // Preferences are a convenience; nothing else depends on them.
    }
  }, [key, value]);
  return [value, setValue];
}
