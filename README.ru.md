# Claude Code Account Switcher

Привязка разных аккаунтов Claude Code к разным директориям.
При `cd` автоматически подхватывается нужный аккаунт.

Кроссплатформенный: macOS, Linux, Windows. Поддержка zsh, bash, PowerShell.

## Установка

### Из бинарника

Скачайте из [GitHub Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases) и добавьте в конфиг шелла:

```bash
# zsh (~/.zshrc)
eval "$(claude-acc init zsh)"

# bash (~/.bashrc)
eval "$(claude-acc init bash)"
```

```powershell
# PowerShell ($PROFILE)
Invoke-Expression (& claude-acc init pwsh)
```

### Из исходников

```bash
cargo install --path .
```

### Legacy (только zsh-скрипт)

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
| `claude-acc init <shell>` | Вывести интеграцию для шелла (zsh/bash/pwsh) |

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

## Лицензия

MIT
