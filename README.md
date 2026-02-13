# Claude Code Account Switcher (macOS)

Привязка разных аккаунтов Claude Code к разным git-репозиториям.
Работает из консоли — при `cd` в репо автоматически подхватывается нужный аккаунт.

## Установка

```bash
cp claude-switch.sh ~/.claude-switch.sh
echo 'source ~/.claude-switch.sh' >> ~/.zshrc
source ~/.zshrc
```

## Быстрый старт

```bash
# 1. Добавьте аккаунты (откроется логин Claude)
claude-add personal
claude-add work

# 2. Задайте дефолтный
claude-default personal

# 3. Привяжите рабочий аккаунт к рабочим репо
cd ~/work/project-alpha
claude-use work

cd ~/work/project-beta
claude-use work

# Готово! При cd в эти репо Claude Code автоматически
# использует рабочий аккаунт, а везде остальное — личный.
```

## Команды

| Команда | Описание |
| --- | --- |
| `claude-accounts` | Список всех аккаунтов |
| `claude-add <имя>` | Добавить аккаунт (запустит `claude login`) |
| `claude-remove <имя>` | Удалить аккаунт |
| `claude-default <имя>` | Задать аккаунт по умолчанию |
| `claude-use <имя>` | Привязать аккаунт к текущему git-репо |
| `claude-unuse` | Убрать привязку с текущего репо |
| `claude-which` | Показать, какой аккаунт сейчас активен |

## Как это работает

```
~/.claude-switch/
├── accounts/
│   ├── personal/    ← конфиг Claude для личного аккаунта
│   └── work/        ← конфиг Claude для рабочего аккаунта
├── config           ← default=personal
└── repos            ← привязки: путь_к_репо=аккаунт
```

Скрипт вешает хук на `cd` (zsh `chpwd`). При смене директории:

1. Определяет корень git-репо
2. Ищет привязку в `~/.claude-switch/repos`
3. Если привязки нет — берёт дефолтный аккаунт
4. Устанавливает `CLAUDE_CONFIG_DIR`

## Пример сессии

```bash
$ claude-which
Активный аккаунт: personal (по умолчанию)

$ cd ~/work/secret-project
$ claude-use work
Репо secret-project → аккаунт 'work'

$ claude-which
Активный аккаунт: work (привязан к secret-project)

$ cd ~/hobby/my-bot
$ claude-which
Активный аккаунт: personal (по умолчанию)
```

## Лицензия

MIT
