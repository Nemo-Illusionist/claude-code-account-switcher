# Статус-панель

[← README](../../README.ru.md) · [English](../statusline.md)

Claude Code умеет показывать настраиваемую панель внизу экрана. `claude-acc statusline` рисует её так, что **первым идёт аккаунт, под которым запущена сессия** — то, чего сам Claude Code показать не может — затем git-ветка, модель, проект и бар 5-часового лимита:

```
work │ ⎇ main │ Opus 4.8 (1M context) │ approvalmax-product-AM-37583 │ ▓▓▓░░░░░░░ 32%
```

Установить в `settings.json` активного аккаунта одной командой:

```bash
claude-acc statusline --install
```

Потом перезапустите Claude Code. Команда читает session-JSON Claude Code со stdin — данные бесплатные, без запросов к API. Бар показывает `rate_limits.five_hour.used_percentage` (живой лимит подписки, который Claude Code отдаёт для Pro/Max), с цветом зелёный → жёлтый → красный по мере приближения к стене; в начале сессии, пока Claude Code его не заполнил, бар не показывается. Имя аккаунта берётся из `CLAUDE_CONFIG_DIR`. Цвета уважают `NO_COLOR`.

Хотите прописать вручную? Укажите в `statusLine` установленный бинарник:

```json
{
  "statusLine": { "type": "command", "command": "~/.claude-switch/bin/claude-acc statusline" }
}
```

> Статус-панель — фича Rust CLI: `statusLine` в Claude Code запускает путь к бинарнику/скрипту, а shell-дистрибуция не может дать его как sourced-функцию.
