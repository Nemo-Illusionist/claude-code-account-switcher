import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../../../", import.meta.url));
const target = execFileSync("rustc", ["--print", "host-tuple"], {
  encoding: "utf8",
}).trim();
execFileSync(
  "cargo",
  ["build", "--release", "--manifest-path", `${root}Cargo.toml`],
  { stdio: "inherit" },
);
const extension = process.platform === "win32" ? ".exe" : "";
const binaries = new URL("../src-tauri/binaries/", import.meta.url);
mkdirSync(binaries, { recursive: true });
copyFileSync(
  `${root}target/release/claude-acc${extension}`,
  new URL(`claude-acc-${target}${extension}`, binaries),
);

assert.match(
  execFileSync(
    fileURLToPath(new URL(`claude-acc-${target}${extension}`, binaries)),
    ["--version"],
    { encoding: "utf8" },
  ).trim(),
  /^claude-acc \d+\.\d+\.\d+$/,
);
