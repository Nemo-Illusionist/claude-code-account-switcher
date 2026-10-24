# Контракт менеджера

[English](../../manager/contract.md)

Что сторона Rust отправляет окну и как обе стороны остаются согласованными. В этом репозитории `claude_acc::gui` можно менять всякий раз, когда это нужно CLI; честность сторон обеспечивает не стабильность, а тест, падающий на каждой стороне при переименовании поля.

## Где он описан

- **Библиотека:** `claude_acc::gui` в `src/gui.rs`. Форма данных закреплена файлом `src/gui.contract.json`, который тест Rust сравнивает с тем, во что сериализуются типы. Документация модуля — письменное описание.
- **Оболочка:** `apps/manager/src-tauri/src/{backend,launcher}.rs` добавляет `revision` snapshot, результат запуска, диагностику установки и свои коды ошибок.
- **Окно:** `apps/manager/src/contract.ts` повторяет всё это на TypeScript. `apps/manager/src/contract.sample.json` проверяется тестами оболочки и фронтенда — с типизированной копией и поле за полем с `src/gui.contract.json`.

## Аккаунты

`snapshot(live)` возвращает `revision`, `observed_at` (Unix seconds), `default_account` и `accounts`, стандартный первым. Оболочка строит snapshot по одному и нумерует их. Окно применяет ответ или событие `snapshot`, отправляемое после каждой сборки, включая обновление из строки меню, только если его `revision` не меньше показанного.

Поля аккаунта: `name`, nullable `email`, `plan` и `organization`, `is_default`, `available`, `identity_source` (`local`, `live`, `unknown`, `offline`, `no_credentials`), `lock` (`matched`, `drift`, `unpinned`, `unknown`, `corrupt`) и `usage`. Совпадение pin не доказывает действующий вход.

`usage.source` — `live`, `cached`, `no_credentials` или `unavailable`. `fetched_at` — момент снятия показаний в Unix seconds либо null. `five_hour` и `seven_day` — null или `{percent, resets_at}`, где `resets_at` — nullable Unix seconds. Окно считает возраст показания по своим часам: десять минут оно live или cached, затем устаревшее. Окно, чей `resets_at` прошёл, не показывает числа, каким бы ни был источник.

`set_default(account)` возвращает обновлённый snapshot, сохраняя показания, за которые уже заплатило предыдущее обновление.

`remove_account(account)` перемещает директорию управляемого аккаунта в корзину, затем сбрасывает default, если он указывал на этот аккаунт, удаляет его привязки и возвращает обновлённый snapshot. Стандартный аккаунт отклоняется. Если перемещение не удалось, ответ — `trash_unavailable`, и ничего не изменено; в отличие от `claude-acc remove`, удаления напрямую не бывает.

`pin_account(account, force)` закрепляет аккаунт за личностью, под которой он вошёл, по правилам `claude-acc lock`, и возвращает обновлённый snapshot. Повторное закрепление той же личности ничего не делает. Pin на другую личность или нечитаемый pin заменяется только с `force`; без него ответ — `identity_drift`. Аккаунт, ни разу не входивший, получает `not_signed_in`. Каждый snapshot, который получает окно, заново читает состояние pin каждого аккаунта, поэтому обновление, начатое раньше, не покажет старое.

## Привязки, сессии и запуск

- `list_bindings(directory?)` возвращает отсортированные `bindings` (`directory`, `account`, `exists`) и optional `selected`: физическую директорию, её действующий `account` и nullable `inherited_from`, разрешённые собственными `resolve_account` и `find_linked_dir` CLI.
- `set_binding(directory, account?)` сохраняет привязку, а при null account удаляет точную сохранённую запись, даже для отсутствующей директории. «default» означает стандартный аккаунт, а не настроенный именованный default.
- `choose_directory()` открывает системный выбор папки.
- `list_sessions()` возвращает метаданные (`id`, `account`, `project`, `modified`, `size`, `duplicate`), новые первыми. Без путей и содержимого transcript.
- `open_terminal(account, directory?, action)` принимает `run`, `login` или `add`. `gui::check_launch` решает, можно ли выполнить действие: корректное имя, безопасное существующее хранилище (для `add` — свободное имя) и, для `run`, отсутствие расходящегося или нечитаемого pin. Ответ — `{state}`: `started`, когда Terminal выполнил скрипт запуска, или `pending`, если не выполнил за десять секунд. Ни то, ни другое не означает, что Claude Code вошёл в аккаунт.
- `installation()` возвращает `bundled_version`, `bundled_available` и nullable версию отдельно установленного CLI. Обе считываются запуском бинарников — при первом открытии Details и снова после live-обновления.

## Ошибки

Каждый сбой — машинный код, который окно переводит в текст: коды библиотеки (`invalid_account`, `account_missing`, `account_exists`, `symlink_storage`, `storage_unreadable`, `storage_write_failed`, `invalid_directory`, `unrepresentable_directory`, `directory_missing`, `shared_credential_store`, `identity_drift`, `trash_unavailable`, `not_signed_in`) и оболочки (`cli_missing`, `launcher_failed`, `terminal_failed`, `terminal_macos_only`, `backend_failed`). Код не несёт путей, вывода процессов и учётных данных. У каждого кода есть текст на каждом языке, и нет текста для кода, который нельзя отправить; `copy.test.ts` проверяет оба направления.

## Никогда не отправляется

Никакие токены, хэши токенов, файлы учётных данных, записи Keychain, сырые файлы конфигурации, пути и содержимое transcript. У типов нет поля, способного их содержать, а тест подкладывает файл учётных данных, лишние секретные поля в `.claude.json` и хэш токена в кэше профиля и проверяет, что ничего из этого не попадает в сериализованный snapshot.
