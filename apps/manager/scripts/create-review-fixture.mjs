// Disposable filesystem records for native QA, never bundled into the application.
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(
  new URL("../../../artifacts/review-home/", import.meta.url),
);
assert(
  !existsSync(root),
  "Fixture already exists; use the existing review home instead of replacing it.",
);
mkdirSync(root, { recursive: true });
const base = join(root, ".claude-switch");
const project = join(root, "Projects", "Studio");
const child = join(project, "Research");
mkdirSync(child, { recursive: true });
for (const [name, email, plan, five, seven] of [
  ["work", "developer@example.test", "Pro", 62, 28],
  ["personal", "personal@example.test", "Max", 17, 9],
]) {
  const dir = join(base, "accounts", name);
  mkdirSync(dir, { recursive: true });
  const uuid = `fixture-${name}`;
  writeFileSync(
    join(dir, ".claude.json"),
    JSON.stringify({
      oauthAccount: { accountUuid: uuid, emailAddress: email },
      cachedUsageUtilization: {
        accountUuid: uuid,
        fetchedAtMs: Date.now() - 900000,
        utilization: {
          five_hour: {
            utilization: five,
            resets_at: new Date(Date.now() + 7200000).toISOString(),
          },
          seven_day: {
            utilization: seven,
            resets_at: new Date(Date.now() + 172800000).toISOString(),
          },
        },
      },
    }),
  );
  writeFileSync(
    join(dir, ".account-info.json"),
    JSON.stringify({
      uuid,
      email,
      plan,
      fetched_at: Math.floor(Date.now() / 1000) - 900,
    }),
  );
  writeFileSync(
    join(dir, ".identity-lock.json"),
    JSON.stringify({ uuid, email }),
  );
  const sessions = join(dir, "projects", "-fixture-Projects-Studio");
  mkdirSync(sessions, { recursive: true });
  writeFileSync(
    join(sessions, "11111111-2222-3333-4444-555555555555.jsonl"),
    JSON.stringify({
      type: "fixture",
      message: "Disposable review session, not an owner conversation.",
    }) + "\n",
  );
}
writeFileSync(join(base, "config"), "default=work\n");
writeFileSync(join(base, "links"), `${project}=work\n${child}=personal\n`);
// Standard account deliberately absent: fixture live refresh must not try the legacy standard Keychain fallback.
assert(!existsSync(join(root, ".claude")));
console.log(root);
