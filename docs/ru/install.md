# Установка и платформы

[← README](../../README.ru.md) · [English](../install.md)

Всё, что не поместилось в три команды из [README](../../README.ru.md#установка):
сборка из исходников, дополнительные шаги на Windows, zsh-скрипт,
автодополнение и переезд между двумя дистрибуциями.

## Из исходников

```bash
cargo install --path .
claude-acc install
```

## Windows

На свежей Windows-машине PowerShell нужны два дополнительных шага, иначе `claude-acc` не заработает:

1. **Разрешить запуск профиля.** Дефолтная execution-policy блокирует PowerShell-профиль, так что shell-init строка, которую мы туда пишем, не выполнится — а именно она кладёт `~/.claude-switch/bin` в `PATH` сессии:
   ```powershell
   Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
   ```
2. **Запустить `install` по полному пути.** Bin-директория ещё не в `PATH`, так что вызываем скачанный `.exe` напрямую:
   ```powershell
   & "$HOME\Downloads\claude-acc.exe" install
   ```
3. **Перезапустить PowerShell.** Профиль выполняется только при старте шелла, поэтому новый `PATH` (и `cd`-активация) подхватываются только в новых процессах. После рестарта `claude-acc add work` работает откуда угодно.

Если поймали старый сломанный install (бинарник без `.exe` или строка для bash в профиле) — просто перезапустите `claude-acc install`. Он сам почистит безрасширенный бинарник и перепишет строку профиля под PowerShell.

**Логин на Windows.** `claude-acc add <имя>` и `claude-acc login <имя>` оба спавнят `claude auth login` под новый `CLAUDE_CONFIG_DIR`. На Windows эта подкоманда сваливается в plain-text режим (без TUI), а OAuth-callback localhost обычно отрабатывает быстрее, чем пользователь успевает вставить код вручную — приглашение `Paste code here if prompted >` ненадёжно. Обход: после того как `claude-acc add <имя>` создал директорию аккаунта, пройти логин через стандартный first-launch TUI самого Claude Code:

```powershell
claude-acc run <имя>
```

Это запустит `claude` напрямую под `CLAUDE_CONFIG_DIR` аккаунта и откроет стандартный welcome → `Select login method:`. Логин в TUI принимает код корректно и пишет credentials в `~/.claude-switch/accounts/<имя>/`. Проверить можно через `claude-acc doctor` — у каждого аккаунта должен быть свой email и UUID.

## Shell-скрипт (только zsh)

```bash
cp claude-switch.sh ~/.claude-switch.sh
echo 'source ~/.claude-switch.sh' >> ~/.zshrc
source ~/.zshrc
```

## Автодополнение в оболочке

`claude-acc install` заодно настраивает автодополнение по Tab для zsh, bash и PowerShell. Оно покрывает все команды и их аргументы — имена аккаунтов (с `default` там, где команда его принимает), для `session copy` — имена живых сессий и id сессий текущей директории, имена профилей `desktop`, `vscode install|uninstall|status`, `resume-hook on|off`, путь для `import` и флаги каждой команды:

```
$ claude-acc session copy <TAB>
notes-api-3f  363edaeb-e81c-4021-94f4-7fe7d91815f4  0266a566-0336-4055-8f05-c553d368528e

$ claude-acc session copy 0266a566-… --to <TAB>
default  personal  work
```

Id сессий намеренно ограничены текущей директорией: полный список — это сотни uuid по всем проектам, которые вы когда-либо открывали, и выбрать из такого меню невозможно.

## Переключение между Rust и shell

Оба варианта читают и пишут одни и те же файлы в `~/.claude-switch/`:

```
~/.claude-switch/
├── accounts/        ← CLAUDE_CONFIG_DIR для каждого аккаунта
├── desktop/         ← user-data-папки профилей Claude Desktop
├── config           ← дефолтный аккаунт
└── links            ← привязки директория ↔ аккаунт
```

Поэтому переключаться можно без пересоздания аккаунтов и перепривязок. Шаги:

**Shell → Rust:**
1. Установить Rust-бинарник: скачать из [Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases) и запустить `claude-acc install`. Эта команда сама добавит свою shell-init строку.
2. Удалить строку `source ~/.claude-switch.sh` из `~/.zshrc` (за активацию теперь отвечает Rust-init).
3. По желанию — `rm ~/.claude-switch.sh`.

**Rust → shell:**
1. `cp claude-switch.sh ~/.claude-switch.sh`, добавить `source ~/.claude-switch.sh` в `~/.zshrc`.
2. Удалить строку `eval "$(... claude-acc init zsh)"` из `~/.zshrc`.
3. По желанию — `rm ~/.claude-switch/bin/claude-acc ~/.claude-switch/bin/claude` (wrapper). Shell-версия пересоздаст свой wrapper при `source`.

Учётные данные аккаунтов, привязки и дефолт сохраняются как есть.
