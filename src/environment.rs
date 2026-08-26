// Env vars that let a caller point Claude Code at a *different* identity
// than CLAUDE_CONFIG_DIR selects — an API key, an OAuth token, or a Bedrock
// bearer token override which credentials are actually used, regardless of
// which account's config dir the process was launched with. If any of these
// leak in from the parent shell, `claude-acc run <name>` can silently target
// the wrong account even though CLAUDE_CONFIG_DIR itself is set correctly.
//
// Mirrors Orca's CLAUDE_AUTH_ENV_VARS (github.com/stablyai/orca,
// src/main/claude-accounts/environment.ts) — same defensive stripping, ported
// here after tracing the same class of bug.
pub const CLAUDE_AUTH_ENV_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "AWS_BEARER_TOKEN_BEDROCK",
];

/// Removes every var in [`CLAUDE_AUTH_ENV_VARS`] from `cmd`'s environment so
/// none of them can override the identity `CLAUDE_CONFIG_DIR` selects.
pub fn strip_claude_auth_env(cmd: &mut std::process::Command) {
    for var in CLAUDE_AUTH_ENV_VARS {
        cmd.env_remove(var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_claude_auth_env_removes_every_listed_var() {
        let mut cmd = std::process::Command::new("true");
        for var in CLAUDE_AUTH_ENV_VARS {
            cmd.env(var, "leaked-value");
        }
        strip_claude_auth_env(&mut cmd);

        for var in CLAUDE_AUTH_ENV_VARS {
            let removed = cmd
                .get_envs()
                .find(|(k, _)| *k == std::ffi::OsStr::new(var));
            assert_eq!(removed, Some((std::ffi::OsStr::new(*var), None)));
        }
    }
}
