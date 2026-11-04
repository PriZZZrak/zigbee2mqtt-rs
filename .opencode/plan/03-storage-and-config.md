# 03. Конфигурация, database и двусторонняя миграция

R1 blocker / P0. Inventory: CONFIG-001/002, STORAGE-001, STORAGE-002/003,
CONFIG-003/004. Dependencies: EVID-01/04; интеграция с RT и ZB.

## Источник контракта

- App `lib/util/settings.ts:143–263` (references/write), `settings.schema.json`,
  `settingsMigration.ts`, `yaml.ts`; `test/settings.test.ts` (включая override,
  secret и повторную запись), `settingsMigration.test.ts`.
- Herdsman `src/controller/database.ts:19–123`, `model/device.ts`, `endpoint.ts`,
  `group.ts`, `tstype.ts`; `test/controller.test.ts`, `device.test.ts` и data
  fixtures. До реализации прочитать все соответствующие serializers, а не
  ограничиваться внешним NDJSON синтаксисом.
- Rust `src/config.rs`, `src/database.rs`, `src/devices/mod.rs`, `src/groups.rs`,
  bridge persistence/import/rename/remove/shutdown.

## STORE-01 — lossless configuration document

**Результат:** редактирование одного поддерживаемого параметра не удаляет прочие
параметры upstream. Семантическая переносимость обязательна; byte-for-byte YAML
и сохранение комментариев — отдельный измеряемый UX контракт, не выдуманный parity.

1. Разделить persisted raw document и typed runtime view. Изменения применять к
   документу по field path, сохраняя неизвестные секции, options и key types.
2. Сделать parsing/validation defaults/schema отдельным этапом. Различать
   отсутствующее значение, null, false, пустую строку, inline map, file reference.
3. Поддержать upstream `mqtt.user`, URI scheme/port, HA mapping, options устройств
   и групп; сохранить legacy `username` только через исследованный precedence.
4. Не раскрывать secrets при round-trip и в logs; не превращать resolved URL
   в persisted значение с потерянным `mqtts` scheme. Env overrides исследовать
   по `applyEnvironmentVariables`, а не трактовать `!env` как env без источника.
5. При referenced files сохранять распределение: запись в первый файл, исключая
   ключи поздних файлов согласно upstream; учитывать conflicts и I/O errors.
6. `persist_groups_config`, rename и options используют один document API;
   никаких повторных `serde_yml::from_str::<Config>` с потерей оригинала.

**Acceptance:** импорт полного upstream config → смена имени → повторная запись
→ Node settings read: исходные настройки, unknown fields, secret refs и options
сохранились; повторить цикл 3 раза, в том числе с отдельными devices/groups.
Негативные случаи: missing reference, malformed YAML, read-only fs, ENOSPC,
duplicate names, invalid channel/type, unsupported URI scheme.

## STORE-02 — database identity и fidelity

1. Единый authoritative raw record + typed projection. Связать IEEE→database ID
   при загрузке и первом выделении нового ID, затем сохранять ID в runtime.
2. В `sync_groups` найти существующие records до удаления; сохранять ID/meta,
   endpoint membership и неизвестные group поля. Не смешивать database.id и
   Zigbee groupID.
3. Device type брать из Node Descriptor/record, не из одного powerSource.
   Сопоставить interview states, включая незавершённое состояние, а не всегда
   писать FAILED для `interview_complete=false`.
4. Сохранить согласованность epList/endpoints, endpoint 242 и порядок там, где
   он наблюдаем; clusters/attributes/binds/configuredReportings/meta не терять.
5. Round-trip firmware/software versions, lastSeen, customClusters,
   configure metadata, OTA state и GreenPower после исследования serializers.
   Исправить запись нового `sw_build_id`, не только сохранение old extra.
6. Рассмотреть malformed/duplicate/out-of-range IDs по reference Database.open.
   Не перезаписывать весь storage пустой проекцией после read error.
7. Проверить remove/rejoin/address changes и coordinator entry id collisions;
   unrelated entries не удаляются из-за фильтрации imported devices.

**Acceptance:** Node-produced DB → Rust import/update/flush → Node open и реальное
чтение devices/endpoints/groups; тот же ID после 100 sync; новый join, leave,
rejoin, unknown fields и duplicate-entry handling. Сравнивать не только строки,
но и результаты чтения моделью herdsman.

## STORE-03 — batching, crash consistency, shutdown

500 ms batching — пожелание пользователя, но его точные гарантии нужно определить:
что можно потерять при внезапном отключении, что должно быть durable до response.

1. Dirty revision/generation вместо неотслеживаемого boolean; планировать flush
   через единого владельца состояния. Не держать DashMap guard через await.
2. Coalesce repeated reports; no-op изменения не переписывают весь DB/config.
   Разделить state cache и network metadata: отчёт температуры не причина
   создавать заново все endpoint records.
3. Atomic write во временный файл на той же FS, fsync файла, rename, проверить
   гарантии directory sync для OpenWrt overlayfs и error propagation.
4. Перед SIGTERM/service restart: остановить принятие новых мутаций, завершить
   или отменить операции, flush последней revision, затем завершить процесс.
   Fault injection: write failure должен быть виден и не сбрасывать dirty state.
5. Согласовать cross-file ordering YAML↔database↔state↔backup; не обещать
   транзакцию нескольких файлов, если реализован только одиночный rename.
6. Потеря питания в каждой точке записи: старая или новая корректная версия,
   восстановление tmp, без повреждённого основного файла.

## STORE-04 — migration/rollback kit

1. Offline `inspect`/preflight без обращения к USB: schema/version, файлы,
   coordinator identity, supported device report, compatibility blockers.
2. Перед переходом backup исходных config, db, state, coordinator backup и
   referenced files; уникальный snapshot вне изменяемого каталога приложения.
3. Проверка несовпадения network key/PAN/channel/IEEE **до** любых write NV.
   `database.db` не является полным coordinator backup.
4. R1 acceptance: original stop → snapshot → Rust start same Dongle-P →
   reports/set/get → orderly stop → original start → те же устройства/имена/
   настройки без повторного pairing. Использовать controlled test setup.
5. Downgrade converters/runtime вместе с versioned cache, storage format version
   и сохранённой конфигурацией. Ручное восстановление gh-pages не требуется.

## Модульная граница

Рекомендуемые будущие подмодули: `config/document.rs`, `config/validation.rs`,
`database/record.rs`, `database/store.rs`, `database/migration.rs`. Это план
декомпозиции, не обязательная массовая перестановка файлов. Сохранить публичные
entry points, выполнить storage correctness до косметического refactor.

**Выход R1:** все STORE-01..04 cases из acceptance matrix подтверждены; ошибки
записи не маскируются `ok`; migration и rollback проверены на Dongle-P.
