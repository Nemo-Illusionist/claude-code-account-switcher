# Claude Code Account Switcher

Привязка разных аккаунтов Claude Code к разным директориям.
При `cd` автоматически подхватывается нужный аккаунт.

Два варианта дистрибуции:

- **Rust CLI** (`claude-acc`) — кроссплатформенный: macOS, Linux, Windows; zsh, bash, PowerShell. **Рекомендуется.**
- **Shell-скрипт** (`claude-switch.sh`) — только zsh, ориентирован на macOS. Один файл, без бинарника и компиляции.

Оба варианта используют одинаковый on-disk формат (`~/.claude-switch/`), так что переключаться между ними можно без миграции.

## Установка

### Rust CLI (рекомендуется)

Скачайте из [GitHub Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases), затем:

```bash
claude-acc install
```

Это:
- Скопирует бинарник в `~/.claude-switch/bin/claude-acc`
- Установит IDE-wrapper в `~/.claude-switch/bin/claude` (см. [Интеграция с IDE](#интеграция-с-ide))
- Определит ваш шелл (zsh/bash/PowerShell)
- Добавит интеграцию в rc-файл

Для обновления — скачайте новую версию и снова запустите `claude-acc install`.

#### Из исходников

```bash
cargo install --path .
claude-acc install
```

### Shell-скрипт (только zsh)

```bash
cp claude-switch.sh ~/.claude-switch.sh
echo 'source ~/.claude-switch.sh' >> ~/.zshrc
source ~/.zshrc
```

## Быстрый старт

```bash
# 1. Добавьте аккаунт (откроется логин Claude)
claude-acc add work

# 2. Привяжите рабочий аккаунт к рабочей папке
cd ~/work
claude-acc link work

# Готово! При cd в ~/work или любую вложенную папку
# используется рабочий аккаунт.
# Всё остальное работает через стандартный ~/.claude/
```

## Команды

| Команда | Описание |
| --- | --- |
| `claude-acc` | Справка |
| `claude-acc list` | Список всех аккаунтов |
| `claude-acc add <имя>` | Добавить аккаунт (запустит `claude login`) |
| `claude-acc login <имя>` | Перелогиниться в аккаунт |
| `claude-acc remove <имя>` | Удалить аккаунт |
| `claude-acc default [имя]` | Показать/задать дефолтный аккаунт |
| `claude-acc reset` | Сбросить дефолт на `~/.claude/` |
| `claude-acc link <имя>` | Привязать аккаунт к текущей директории |
| `claude-acc unlink` | Убрать привязку с текущей директории |
| `claude-acc links` | Показать все привязки директорий |
| `claude-acc status` | Показать активный аккаунт |
| `claude-acc run <имя>` | Запустить claude под конкретным аккаунтом |
| `claude-acc doctor` | Аудит реальной OAuth-личности каждого аккаунта |
| `claude-acc install` | Установить бинарник и shell-интеграцию |

## Как это работает

```
~/.claude-switch/
├── accounts/
│   └── work/        ← конфиг Claude для рабочего аккаунта
├── config           ← default=work (или пусто для ~/.claude/)
└── links            ← привязки: путь=аккаунт
```

При смене директории:

1. Ищет привязку для текущей директории в `~/.claude-switch/links`
2. Если нет — поднимается вверх по дереву директорий
3. Если привязки не найдено — берёт дефолтный аккаунт (или `~/.claude/`)
4. Устанавливает `CLAUDE_CONFIG_DIR`

## Наследование директорий

Привязка распространяется на **все вложенные папки** автоматически.
Не нужно привязывать каждый проект отдельно:

```
~/work                  → work      (привязан явно)
~/work/project-a        → work      (унаследовано)
~/work/project-b        → work      (унаследовано)
~/work/project-b/src    → work      (унаследовано)
~/personal              → ~/.claude/ (по умолчанию)
```

Более конкретная привязка всегда побеждает. Это позволяет задавать исключения:

```
~/work                  → work      (привязан)
~/work/project-a        → work      (унаследовано)
~/work/secret           → personal  (привязан — перекрывает родителя)
~/work/secret/src       → personal  (унаследовано от secret)
```

Используйте `default` как зарезервированное имя, чтобы явно вернуться к `~/.claude/`:

```
~/work                  → work      (привязан)
~/work/project-a        → work      (унаследовано)
~/work/hobby            → ~/.claude/ (привязан к default — перекрывает родителя)
~/work/hobby/sub        → ~/.claude/ (унаследовано от hobby)
```

```bash
cd ~/work/hobby
claude-acc link default
# hobby → ~/.claude/ (default)
```

## Интеграция с IDE

JetBrains IDE (PhpStorm, IntelliJ и т.п.) и VSCode запускают `claude` напрямую, не source-я ваш shell-конфиг. Без этого `CLAUDE_CONFIG_DIR` не выставится и подхватится не тот аккаунт. Чтобы это работало для не-default аккаунтов, `claude-acc install` ставит две вещи:

- Wrapper `~/.claude-switch/bin/claude`, который определяет аккаунт для текущей директории (через `claude-acc activate`) и `exec`-ает реальный `claude`. `~/.claude-switch/bin` добавляется в начало `PATH` (через shell-init), так что и терминал, и IDE прозрачно подхватывают wrapper.
- Symlink `~/.claude-switch/accounts/<name>/ide → ~/.claude/ide` для каждого аккаунта. Claude Code пишет lock-файлы IDE в `$CLAUDE_CONFIG_DIR/ide/`, а IDE-плагины ищут их в `~/.claude/ide/`. Symlink приводит обе стороны к одному месту.

Никаких ручных шагов не нужно — `claude-acc install` делает обе вещи. Новые аккаунты, создаваемые через `claude-acc add`, получают `ide/` symlink автоматически.

## Что переключается

`CLAUDE_CONFIG_DIR` перемещает всю директорию `~/.claude/`, включая ([docs](https://code.claude.com/docs/en/settings)):

| Файл | Описание |
|---|---|
| `settings.json` | Пользовательские настройки |
| `CLAUDE.md` | Глобальная память / инструкции |
| `agents/` | Субагенты |
| `.credentials.json` | Данные авторизации |
| `projects/` | Глобальные конфиги по проектам |
| sessions, history и т.д. | Runtime-данные |

Каждый аккаунт получает свою копию всех этих файлов в `~/.claude-switch/accounts/<name>/`.

## Отдельные настройки для проектов

Каждый аккаунт получает свою папку `~/.claude-switch/accounts/<name>/`, которая используется как `CLAUDE_CONFIG_DIR`. Это значит, что у каждого аккаунта свои `settings.json`, credentials и история проектов.

Это можно использовать для разных настроек на разных проектах — даже под одной авторизацией. Просто создайте несколько аккаунтов и войдите с теми же данными:

```bash
# Общий рабочий аккаунт с дефолтными настройками
claude-acc add work
cd ~/work
claude-acc link work

# Тот же логин, но со своими настройками для конкретного проекта
claude-acc add work-ml
cd ~/work/ml-project
claude-acc link work-ml

# Теперь можно настраивать settings независимо:
# ~/.claude-switch/accounts/work/settings.json       — для всех рабочих проектов
# ~/.claude-switch/accounts/work-ml/settings.json     — только для ml-project
```

> Примечание: `claude-acc add` запускает `claude login`, поэтому нужно будет залогиниться повторно (тот же аккаунт, просто новая папка конфига).

## Аудит личностей (`doctor`)

`claude-acc add` и `claude-acc login` оба запускают `claude auth login` под персональным `CLAUDE_CONFIG_DIR`. С каким Anthropic-аккаунтом вы залогинились — тот и привязан к этой директории. Встроенного способа увидеть, *какой именно* аккаунт реально стоит за config dir, нет. Если случайно залогинились не той учёткой (browser auto-fill, забытая вкладка), переключение происходит молча: лимиты, история диалогов и биллинг перепутаются между «изолированными» аккаунтами без всяких признаков.

`claude-acc doctor` читает OAuth-токен каждого аккаунта из macOS Keychain (с fallback на `.credentials.json` для установок без Keychain), дёргает `https://api.anthropic.com/api/oauth/profile` и печатает реальный email + UUID:

```
$ claude-acc doctor
Проверка 2 аккаунт(ов):
  ✓ work      alice@anthropic.com  uuid=aa6c22d5-…
  ? personal  нет токена (запустите: claude-acc login personal)

1 из 2 аккаунтов в порядке.
```

Это исключительно read-only аудит — ничего не перехватывается, запуск `claude` не блокируется. Запускайте когда хочется убедиться, что за config dir стоит ожидаемая личность. Требует `security`, `curl`, `jq`, `shasum` (всё предустановлено на macOS); Rust-бинарник использует нативные `serde_json` и `sha2`, шеллаутит только `security` и `curl`.

> **Пока только macOS.** Схема хеширования Keychain reverse-engineered из внутренностей Claude Code; на других платформах (где используется libsecret / Credential Manager) пока не работает.

## Язык

Определяется автоматически из `LANG`. Можно задать вручную:

```bash
export CLAUDE_ACC_LANG=ru  # или en
```

## Пример сессии

```bash
$ claude-acc status
Активный аккаунт: ~/.claude/ (стандартный)

$ claude-acc add work
Аккаунт 'work' создан. Запускаю логин...

$ cd ~/work
$ claude-acc link work
work → аккаунт 'work'

$ cd ~/work/secret-project
$ claude-acc status
Активный аккаунт: work (привязан к work)

$ cd ~/hobby/my-bot
$ claude-acc status
Активный аккаунт: ~/.claude/ (стандартный)
```

## Переключение между Rust и shell

Оба варианта читают и пишут одни и те же файлы в `~/.claude-switch/`:

```
~/.claude-switch/
├── accounts/        ← CLAUDE_CONFIG_DIR для каждого аккаунта
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

## Релизы

Релизы автоматизированы через [release-please](https://github.com/googleapis/release-please). На каждый push в `master` action читает [conventional commits](https://www.conventionalcommits.org/ru/v1.0.0/) и держит открытой одну "Release PR" с бампом версии и changelog. Мерж этой PR создаёт тег и собирает кроссплатформенные бинарники (macOS x64/arm64, Linux x64/arm64, Windows x64), приклеивая их к релизу.

Префиксы коммитов для корректного bump:

| Префикс | Bump |
|---|---|
| `feat:` | minor (`0.1.0 → 0.2.0`) |
| `fix:` / `perf:` / `refactor:` / `docs:` | patch (`0.1.0 → 0.1.1`) |
| `feat!:` или `BREAKING CHANGE:` в теле | major (`0.1.0 → 1.0.0`) |
| `chore:` / `ci:` / `build:` / `style:` / `test:` | без релиза |

## Лицензия

MIT
