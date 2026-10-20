# Desktop-контракт

[English](../../gui/contract.md)

JSON между Rust-бэкендом и React-фронтендом описан по одному разу на каждой стороне и закреплён одним образцом. Rust — типы с `Serialize` в `src/gui.rs` и `apps/desktop/src-tauri/src/{backend,launcher}.rs`. TypeScript — `apps/desktop/src/contract.ts`. Обе стороны проверяются тестами против `apps/desktop/src/contract.sample.json`, поэтому переименованное или изменённое по типу поле роняет тест на своей стороне. Версии схемы нет: фронтенд и бэкенд поставляются одним бандлом.

Ошибки — машинные коды (`GuiError` в библиотеке, `AppError` в desktop-крейте), которые фронтенд переводит в текст. Код не несёт путей, вывода процессов и учётных данных. Для каждого кода есть текст, и нет текста для кода, который нельзя отправить: `contract.test.ts` проверяет оба направления.

## Аккаунты

`snapshot(live)` возвращает `revision`, `observed_at` (Unix seconds), `default_account` и `accounts`, стандартный первым. Бэкенд строит snapshot по одному и нумерует их. Ответы и события `snapshot` (отправляются после каждой сборки, включая обновление из трея) применяются, только если их `revision` не меньше показанного.

Поля аккаунта: `name`, nullable `email`, `plan`, `organization`, booleans `is_default` и `available`, `identity_source` (`local`, `live`, `unknown`, `offline`, `no_credentials`), `lock` (`matched`, `drift`, `unpinned`, `unknown`, `corrupt`) и `usage`. Совпадение локального pin не подтверждает сетевой вход.

`usage.source` — `live`, `cached`, `no_credentials` или `unavailable`. `fetched_at` — момент снятия показаний в Unix seconds, либо null, если их нет. Фронтенд считает возраст по своим часам: до десяти минут показания live или cached по источнику, дальше — устаревшие. `five_hour` и `seven_day` — null или `{percent, resets_at}`, где `resets_at` — nullable Unix seconds. Когда `resets_at` прошёл, процент не показывается, каким бы ни был источник.

`set_default(account)` возвращает обновлённый snapshot. Показания, за которые уже заплатило предыдущее обновление, сохраняются; меняется только default.

## Привязки и изменения

`list_bindings(directory?)` возвращает отсортированные `bindings` (`directory`, `account`, `exists`) и optional `selected`: физический каталог, действующий `account` и nullable `inherited_from`, разрешённые теми же `resolve_account` и `find_linked_dir`, что CLI. `set_binding(directory, account?)` сохраняет привязку или, при null `account`, удаляет точную сохранённую запись, включая отсутствующий каталог. «default» означает стандартный аккаунт, а не настроенный именованный default. `choose_directory()` открывает системный выбор папки.

Запись в `~/.claude-switch/config` и `links` держит эксклюзивную блокировку `~/.claude-switch/.config.lock` на всё чтение-изменение-запись и заменяет файл через rename. CLI, приложение и `claude-switch.sh` берут одну и ту же блокировку. Скрипт использует `zsystem flock` из zsh, который на macOS исключает `File::lock` из Rust. Если блокировку не удаётся получить за пять секунд, скрипт выполняет прежнюю запись без блокировки.

## Запуск и диагностика

`open_terminal(account, directory?, action)` принимает `action` — `run`, `login` или `add`. `gui::check_launch` решает, можно ли выполнить действие: корректное имя, существующее безопасное хранилище (для `add` — свободное имя) и, для `run`, отсутствие drift или повреждённого pin. Возвращает физический каталог, по умолчанию домашний. Ответ — `{state}`: `started`, когда лаунчер выполнился в Terminal и передал управление встроенному CLI, или `pending`, если Terminal принял запрос, но не выполнил его за десять секунд. Ни то, ни другое не означает, что Claude Code вошёл в аккаунт: его вывод — в окне Terminal. Запуск — только macOS.

`list_sessions()` возвращает метаданные (`id`, `account`, `project`, `modified`, `size`, `duplicate`), новые первыми. Пути и содержимое transcript не передаются.

`installation()` возвращает `bundled_version`, `bundled_available` и nullable версию отдельно установленного CLI, считанную один раз за запуск. Установленный CLI приложение для своих операций не использует.

## Правила тестирования

Используйте временные корни или отдельный процесс с изолированным HOME. Не тестируйте изменения или реальный вход на аккаунтах владельца. Скриншоты нативного приложения должны использовать явно помеченные fixtures. Существующие Keychain-тесты CLI используют отдельные `claude-acc-test-*` services, а не реальные записи. Единственный тест, который по-настоящему открывает Terminal, запускает CLI-заглушку и помечен `#[ignore]`. Запускайте его вручную: `cargo test --release -- --ignored real_terminal` в `apps/desktop/src-tauri`.
