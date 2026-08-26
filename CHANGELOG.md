# Changelog

## [0.14.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.13.0...v0.14.0) (2026-08-26)


### Features

* check `--resume` in the claude wrapper, with an on/off switch ([#81](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/81)) ([d250251](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/d250251c59435565dd1f5fe59b0bf83c65b77131))
* complete every command's arguments and flags ([#84](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/84)) ([cd5704f](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/cd5704f9852eee53ff876adcbd626a872f7cab89))


### Bug Fixes

* refresh the claude wrapper on update ([#83](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/83)) ([25cd47f](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/25cd47f5dce78fb88a2f52a269882be4b159c8c7))

## [0.13.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.12.0...v0.13.0) (2026-08-26)


### Features

* check cross-account copies before `run --resume` ([#78](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/78)) ([03d4fab](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/03d4fab8be3d6e43b2135ac2a0b2fdb11359679d))
* copy a session into another account ([#77](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/77)) ([3e554eb](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/3e554eb1e397c4b9215b656d0c497a2fe37c27a9))
* list Claude Code sessions across accounts ([#76](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/76)) ([63995f9](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/63995f92d9c69a86aa2cd58eef3016cdc0c46ceb))


### Documentation

* document the session commands in the Russian README ([#80](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/80)) ([798da6a](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/798da6ac164a8c2f98f1e0a4d8525619ca3a5e31))

## [0.12.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.11.0...v0.12.0) (2026-08-26)


### Features

* print a passive hint when a newer claude-acc version is out ([#74](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/74)) ([48cf191](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/48cf191e96b0bfa66279d61f625ef03d2435448f))
* update accepts --version for pinning/rollback ([#72](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/72)) ([952c1c0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/952c1c06a88ffc93813d6441385cb4c6bb6b1f95))

## [0.11.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.5...v0.11.0) (2026-08-26)


### Features

* warn (non-blocking) when add/login duplicates an existing account ([#70](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/70)) ([f42f1ac](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/f42f1ac42161afff74f7264b06ee8cb2cd1f94e4))


### Bug Fixes

* doctor/usage fall back to the legacy Keychain entry for ~/.claude ([#68](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/68)) ([2fc3144](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/2fc3144d0abab7197f30c8a1faceb62e0988c0f3))

## [0.10.5](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.4...v0.10.5) (2026-08-26)


### Bug Fixes

* add/login no longer clobber the standard account's Keychain entry ([#62](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/62)) ([dde70e5](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/dde70e5d87bf990baac67137d0d782473229d13e))
* add/login strip auth env vars before claude auth login ([#64](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/64)) ([395f53f](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/395f53f619188b292dcdbc824b18ae3c6ce63f81))
* clone-settings rejects "default" with a clear message ([#67](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/67)) ([66bdf23](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/66bdf23ba3b46e40d70b41a4900478ab835e2115))
* login accepts "default" and completions offer it for login/run ([#65](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/65)) ([eb97ca9](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/eb97ca9cdb253d7db9ba58af1623a6f537225302))
* run strips auth env vars that can override the selected account ([#61](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/61)) ([cd09246](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/cd09246718de011cb74954170cf6c085c4935a7e))

## [0.10.4](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.3...v0.10.4) (2026-08-26)


### Bug Fixes

* run default no longer gets re-activated by the IDE wrapper ([#59](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/59)) ([91c1391](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/91c139131f4191da901c64d64b1316a1e7967ec6))

## [0.10.3](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.2...v0.10.3) (2026-08-25)


### Bug Fixes

* run default no longer inherits a linked account's CLAUDE_CONFIG_DIR ([#58](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/58)) ([be3b487](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/be3b48718e4645479dfa6215cdb603c9d784c28a))


### Documentation

* demo GIF, positioning and comparison sections ([#54](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/54)) ([56a1873](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/56a18737cdbdf8a8829ad13f55aae331653abadc))
* move the positioning section above Install ([#56](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/56)) ([d1fd82b](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/d1fd82b1a377876ba76869a1b97dc0dfe8bb8764))

## [0.10.2](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.1...v0.10.2) (2026-06-15)


### Bug Fixes

* show context-window usage in statusline instead of 5h quota ([#52](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/52)) ([b5f1db5](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/b5f1db5b21b1b2dcd6f2d105c0899d3bb82839e2))

## [0.10.1](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.10.0...v0.10.1) (2026-06-12)


### Bug Fixes

* emit forward-slash statusLine path on Windows ([#49](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/49)) ([#50](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/50)) ([d09b012](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/d09b0126530b01182762a606d2fd375908c9cd49))

## [0.10.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.9.0...v0.10.0) (2026-06-10)


### Features

* add statusline command for a Claude Code status bar ([#47](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/47)) ([bdddfef](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/bdddfef982a83ddc89893736a1bcf16d9f50ee60))

## [0.9.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.8.0...v0.9.0) (2026-06-10)


### Features

* add import command to adopt an existing config dir without re-login ([#45](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/45)) ([34507ed](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/34507ed4f08ef1fcaf1b9d063a2b7aa2625ca2ea))
* add update command for self-updating from GitHub releases ([#44](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/44)) ([3208c4b](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/3208c4b337b0868621b2cbcf02be21a757643757))
* cross-reference accounts that share an identity in doctor ([#42](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/42)) ([88892db](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/88892db0688e056d1930c0ad23b37c6e232a0cc4))

## [0.8.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.7.0...v0.8.0) (2026-06-10)


### Features

* show subscription plan/tier in doctor, list, and usage ([#40](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/40)) ([99c1161](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/99c116191938e2cac3ba54777307df76c9063eef))

## [0.7.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.6.3...v0.7.0) (2026-06-10)


### Features

* add usage command for 5h / 7d rate-limit tracking ([#38](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/38)) ([935eff5](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/935eff5c2eac140755b9cc146f088a33768b3634))

## [0.6.3](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.6.2...v0.6.3) (2026-05-06)


### Documentation

* keep clone-settings help short so commands list stays aligned ([#36](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/36)) ([c50a1d8](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/c50a1d8f4024a6af464912ea90c0559120733223))

## [0.6.2](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.6.1...v0.6.2) (2026-05-06)


### Documentation

* document Windows TUI workaround for first-time account login ([#34](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/34)) ([9643dd7](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/9643dd756b28427085157f7f0596adaca4853042))

## [0.6.1](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.6.0...v0.6.1) (2026-05-06)


### Bug Fixes

* **install:** join `init pwsh` output before Invoke-Expression + Windows install docs ([#32](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/32)) ([657fb8e](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/657fb8e5772c17c9e2a66842b549d2ed46f8483f))

## [0.6.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.5.1...v0.6.0) (2026-05-06)


### Features

* `-s, --seed` flag and `clone-settings` for seeding from ~/.claude/ ([#28](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/28)) ([e7e6e68](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/e7e6e681245c6e0b551cfd2b625b1b1782e32794))

## [0.5.1](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.5.0...v0.5.1) (2026-05-06)


### Bug Fixes

* **install:** correct binary extension and shell detection on Windows ([#29](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/29)) ([d12c124](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/d12c12429b6cbd4f805297bbf4f722c7a98bcda8))

## [0.5.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.4.0...v0.5.0) (2026-05-05)


### Features

* `claude-acc whoami` and `doctor --json` ([#25](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/25)) ([2e93aef](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/2e93aef9c445482f159c4e60cbcce72952f1cbd4))

## [0.4.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.3.0...v0.4.0) (2026-05-05)


### Features

* cache doctor result and show email in `list` / `status` ([#20](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/20)) ([0073e3a](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/0073e3af641c3e41ba303753b0d2d74cc87b6172))
* surface ~/.claude/ standard identity in doctor / list / status / default ([#22](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/22)) ([e1ac28c](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/e1ac28c6ffadf71dd4ab3274e3298de7e4c81302))

## [0.3.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.2.1...v0.3.0) (2026-05-05)


### Features

* `claude-acc doctor` for OAuth identity audit (Phase 1 of identity-lock) ([#14](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/14)) ([eb2de28](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/eb2de28fff8dac45b2474e41b5b9fed22bcf21e3))

## [0.2.1](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.2.0...v0.2.1) (2026-05-05)


### Documentation

* migration guide between Rust and shell distributions ([#11](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/11)) ([82226a1](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/82226a1ae4ed914d3b0070835ed934b0e3f8a2bd))

## [0.2.0](https://github.com/Nemo-Illusionist/claude-code-account-switcher/compare/v0.1.0...v0.2.0) (2026-05-05)


### Features

* add `claude-acc run <name>` to shell version ([#6](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/6)) ([1052812](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/10528122466cb55534d1dd4ee409289925cbe5f3))


### Refactors

* single entry point claude-acc with subcommands and tab completion ([638881c](https://github.com/Nemo-Illusionist/claude-code-account-switcher/commit/638881ced4d13c980bec807873b47c3a50b1fba5))
