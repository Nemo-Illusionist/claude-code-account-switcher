# Claude Code Account Switcher (macOS)

Привязка разных аккаунтов Claude Code к разным директориям.
При `cd` автоматически подхватывается нужный аккаунт.

## Установка

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
| `claude-acc remove <имя>` | Удалить аккаунт |
| `claude-acc default [имя]` | Показать/задать дефолтный аккаунт |
| `claude-acc reset` | Сбросить дефолт на `~/.claude/` |
| `claude-acc link <имя>` | Привязать аккаунт к текущей директории |
| `claude-acc unlink` | Убрать привязку с текущей директории |
| `claude-acc status` | Показать активный аккаунт |

## Как это работает

```
~/.claude-switch/
├── accounts/
│   └── work/        ← конфиг Claude для рабочего аккаунта
├── config           ← default=work (или пусто для ~/.claude/)
└── links            ← привязки: путь=аккаунт
```

При смене директории скрипт:

1. Ищет привязку для текущей директории в `~/.claude-switch/links`
2. Если нет — поднимается вверх по дереву директорий
3. Если привязки не найдено — берёт дефолтный аккаунт (или `~/.claude/`)
4. Устанавливает `CLAUDE_CONFIG_DIR`

Привязав `~/work` к аккаунту `work`, все вложенные папки
(`~/work/project-a`, `~/work/project-b/src`) автоматически унаследуют этот аккаунт.

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
