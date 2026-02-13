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
claude-acc add personal
claude-acc add work

# 2. Задайте дефолтный
claude-acc default personal

# 3. Привяжите рабочий аккаунт к рабочим репо
cd ~/work/project-alpha
claude-acc link work

cd ~/work/project-beta
claude-acc link work

# Готово! При cd в эти репо Claude Code автоматически
# использует рабочий аккаунт, а везде остальное — личный.
```

## Команды

| Команда | Описание |
| --- | --- |
| `claude-acc` | Справка |
| `claude-acc list` | Список всех аккаунтов |
| `claude-acc add <имя>` | Добавить аккаунт (запустит `claude login`) |
| `claude-acc remove <имя>` | Удалить аккаунт |
| `claude-acc default [имя]` | Показать/задать аккаунт по умолчанию |
| `claude-acc link <имя>` | Привязать аккаунт к текущему git-репо |
| `claude-acc unlink` | Убрать привязку с текущего репо |
| `claude-acc status` | Показать, какой аккаунт сейчас активен |

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
$ claude-acc status
Активный аккаунт: personal (по умолчанию)

$ cd ~/work/secret-project
$ claude-acc link work
Репо secret-project → аккаунт 'work'

$ claude-acc status
Активный аккаунт: work (привязан к secret-project)

$ cd ~/hobby/my-bot
$ claude-acc status
Активный аккаунт: personal (по умолчанию)
```

## Лицензия

MIT
