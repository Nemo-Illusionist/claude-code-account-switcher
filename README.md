# Claude Code Account Switcher (macOS)

Привязка разных аккаунтов Claude Code к разным директориям.
Работает из консоли — при `cd` автоматически подхватывается нужный аккаунт.

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

# 3. Привяжите рабочий аккаунт к рабочей папке
cd ~/work
claude-acc link work

# Готово! При cd в ~/work или любую вложенную папку
# Claude Code автоматически использует рабочий аккаунт,
# а везде остальное — личный.
```

## Команды

| Команда | Описание |
| --- | --- |
| `claude-acc` | Справка |
| `claude-acc list` | Список всех аккаунтов |
| `claude-acc add <имя>` | Добавить аккаунт (запустит `claude login`) |
| `claude-acc remove <имя>` | Удалить аккаунт |
| `claude-acc default [имя]` | Показать/задать аккаунт по умолчанию |
| `claude-acc link <имя>` | Привязать аккаунт к текущей директории |
| `claude-acc unlink` | Убрать привязку с текущей директории |
| `claude-acc status` | Показать, какой аккаунт сейчас активен |

## Как это работает

```
~/.claude-switch/
├── accounts/
│   ├── personal/    ← конфиг Claude для личного аккаунта
│   └── work/        ← конфиг Claude для рабочего аккаунта
├── config           ← default=personal
└── links            ← привязки: путь=аккаунт
```

При смене директории скрипт:

1. Ищет привязку для текущей директории в `~/.claude-switch/links`
2. Если нет — поднимается вверх по дереву директорий
3. Если привязки не найдено — берёт дефолтный аккаунт
4. Устанавливает `CLAUDE_CONFIG_DIR`

Это значит, что привязав `~/work` к аккаунту `work`, все вложенные папки
(`~/work/project-a`, `~/work/project-b/src`) автоматически унаследуют этот аккаунт.

## Пример сессии

```bash
$ claude-acc status
Активный аккаунт: personal (по умолчанию)

$ cd ~/work
$ claude-acc link work
work → аккаунт 'work'

$ cd ~/work/secret-project
$ claude-acc status
Активный аккаунт: work (привязан к work)

$ cd ~/hobby/my-bot
$ claude-acc status
Активный аккаунт: personal (по умолчанию)
```

## Лицензия

MIT
