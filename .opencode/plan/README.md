# План завершения zigbee2mqtt-rs

Статус: **план работ, не отчёт о завершённой реализации**. Аудит исходников: Rust
`07bb90f1012e41897ce1d30f79e45a5cfd01ecf2`. Приоритеты уточнены в разговоре
2026-11-04; технические ADR ниже остаются предложениями. Первые проверки среды
датированы 2026-11-03 UTC, итоговый refresh — 2026-11-04T09:08:33.742736+00:00.
Timestamps сохранены без подмены даты. Источники и ограничения:
[`AUDIT-20261104.md`](../../docs/parity/evidence/AUDIT-20261104.md).

## Что именно нужно получить

1. **R1 — первый эксплуатационный релиз:** OpenWrt **24.10.8**, **Routerich AX3000 v1**,
   заявленное пользователем общее место **54 МБ**, координатор **SONOFF Dongle-P**.
   Приоритет: перенос существующей сети из оригинала и обратно, Lytko «L101 NS»,
   существующий датчик температуры и выключатели Moes. Точные Zigbee modelID,
   manufacturerName, endpoints и firmware ещё нужно снять; торговое название не
   является fingerprint. RAM и свободное место пока неизвестны.
   Личные устройства — приоритетные fixtures/HIL для **общего** механизма,
   никаких отдельных персональных путей обработки или исключений из требований.
2. **S1 — полная совместимость Z-Stack с зафиксированным upstream:** все общие
   функции приложения, весь каталог, CC2652P и сохранённая поддержка CC2531.
   Пользователь снял обязательность аппаратной проверки CC2531 **для R1**.
   Это не удаляет CC2531 из общего объёма и не делает R1 завершением S1.
3. **G — глобальная совместимость:** все поддерживаемые pinned upstream адаптеры
   и оставшиеся общие контракты. Будущие радиаторные термостаты включаются в
   персональную матрицу после выбора моделей; до этого их поддержка UNKNOWN.

R1 — явно ограниченный релиз для проверенной сети. Его нельзя называть drop-in
заменой для любого пользователя, «90% Windfront» или «поддержкой 5426 устройств».

## Состояние выполнения

- **PR-01 — выполнено локально** (`a700832`, `f4ef0f0`): baseline/evidence
  зафиксированы, CSV отремонтирован, неподтверждённые `VERIFIED` и case-счётчики
  переоткрыты, добавлены строгий валидатор, генерация dashboard с CI-check и
  Rust/parity workflow. GitHub Actions ещё не запускался.
- **PR-02 — частично выполнено**: STORE-01 (lossless YAML merge, unknown
  options, referenced files), STORE-02 (стабильные group/device database ID,
  epList/interviewState/swBuildId) и STORE-03 (SIGTERM-handler, принудительный
  flush, fsync каталога, видимые ошибки записи) реализованы с локальными
  regression tests. STORE-04 начат: offline `--inspect` и проверяемый
  `--snapshot-dir`; автоматический restore/rollback и реальный HIL-перенос ещё
  не сделаны. SIGTERM/ENOSPC/power-loss и Node/differential oracle отсутствуют,
  поэтому строки `IN_PROGRESS`, а не `VERIFIED`.
- Следующий шаг: завершить STORE-04 (restore/rollback), затем reference harness
  (EVID-04) и Z-Stack resume (ZB-01).
- **PR-03 начат (ZB-01, ZB-02, ZB-03, ZB-04)**: SREQ сериализуются FIFO;
  startup strategy resume/restore/commissioning с NV probe (resume не перезаписывает
  NV, restore отклонён); unknown NWK дедуплицируется; ZCL-транзакции получили waiter
  registry с match по адресу/endpoint/cluster/TSN/command, AF confirm, timeout и
  retry, но `request_zcl` ещё не используется bridge/converters. Recovery, route
  discovery и hardware tests отсутствуют.
- **CV-03 начат**: registry lookup по Zigbee modelID, bucket `null`, NUL-нормализация
  и fingerprint priority реализованы; product `models` больше не используется как
  ключ. Неизвестные fingerprint-поля, реальный пакет и Node oracle ещё не проверены;
  CV-04..08 не начаты.
- **CV-01/02 частично**: workflow сохраняет `devices/`, `converters/`, `lib/`,
  генерирует manifest с SHA-256 и валидирует пакет перед публикацией; добавлены
  Python layout tests. GitHub Actions и Node staged smoke ещё не выполнялись,
  live `latest` до следующего запуска остаётся в старом layout.
- **CV-04 частично**: registry-пути проверяются на traversal/absolute/backslash,
  загрузка ограничена 8 MiB. Digest/ETag, timeout, versioned cache и atomic
  promotion ещё не реализованы.
- **RT-03 частично**: `mqtts://` больше не превращается молча в plain TCP —
  конфигурация помечает TLS-требование, connect возвращает явную ошибку; URI
  сохраняется lossless. TLS provider и `mqtt.user` auth не реализованы.

## Документы

| Документ | Назначение |
|---|---|
| [01-current-state.md](01-current-state.md) | Оценка кода, подтверждённые дефекты, качество тестов |
| [02-parity-and-ci.md](02-parity-and-ci.md) | Достоверный baseline, инвентаризация, oracle, CI |
| [03-storage-and-config.md](03-storage-and-config.md) | Двусторонняя переносимость и сохранность состояния |
| [04-zstack-and-zigbee.md](04-zstack-and-zigbee.md) | Dongle-P, legacy CC2531, transport, interview, ZCL |
| [05-converters.md](05-converters.md) | Actions, registry, cache, Boa, host API, реальные устройства |
| [06-runtime-mqtt.md](06-runtime-mqtt.md) | Event loop, команды MQTT, состояние, reconnect, shutdown |
| [07-windfront-ha-and-features.md](07-windfront-ha-and-features.md) | WebSocket API, UI, HA, groups, scenes, OTA |
| [08-openwrt-and-release.md](08-openwrt-and-release.md) | 54 МБ, упаковка, измерения, миграция, rollout |
| [09-global-parity.md](09-global-parity.md) | Полный S1, остальные адаптеры, долгосрочная поддержка |
| [10-acceptance-matrix.md](10-acceptance-matrix.md) | Сценарии и обязательные доказательства приёмки |
| [11-decisions-and-questions.md](11-decisions-and-questions.md) | Принятые решения и оставшиеся вопросы |

## Очерёдность и зависимости

```text
A0: снимок исходников + исправление учёта + паспорт роутера/устройств
 ├─ A1: config/database round-trip ───────────────┐
 ├─ A2: безопасный resume Dongle-P + ZCL requests ├─ A5: Lytko/Moes/sensor end-to-end
 ├─ A3: reproducible Actions + registry + cache ─┤
 └─ A4: Boa real-package spike + host bridge ────┘
A5 + MQTT lifecycle + минимальный настоящий Windfront API
 → A6: OpenWrt package + recovery + HIL/soak → R1
 → общие группы/scenes/reporting/OTA/HA/config/frontend gaps → полный S1
 → адаптерные стадии + исчерпывающий аудит → G
```

Независимые ветки можно вести параллельно отдельными изменениями. Общие
контракты host API/ZCL requests согласовать до их интеграции. Никакой фоновой
бизнес-логики только ради параллелизма: один владелец состояния bridge.

### Ближайшие reviewable изменения

1. **PR-01 / P0:** `EVID-01..03`: валидатор inventory, реальная версия Windfront,
   доказательства вместо исторических процентов; Rust CI.
2. **PR-02 / P0:** `STORE-01..03`: воспроизводящие round-trip fixtures и сохранение
   неизвестных полей/ссылок YAML; стабильные ID database/group.
3. **PR-03 / P0:** `ZB-01..03`: resume/restore/commissioning и подтверждения ZCL;
   никаких экспериментов со сбросом пользовательской сети.
4. **PR-04 / P0:** `CV-01..03`: layout опубликованного пакета, version manifest,
   staged Node smoke и atomic latest; тест начинает с пустого каталога cache.
5. **PR-05 / P0:** `CV-04..06`: upstream lookup + prepareDefinition + real-package
   Boa spike. Выход — измеренный отчёт о возможностях, не очередной stub.
6. **PR-06 / P0:** `CV-07..09`: общий async host API, real fz/tz/configure;
   первыми проверяются фактический Lytko, затем Moes и датчик пользователя.
7. **PR-07 / P0:** `RT-01..04`: get/set payloads, lifecycle, durable rename/remove,
   устранение ложного успеха команд.
8. **PR-08 / P1:** `UI-01..03`, `HA-PLAN-01`: настоящий WebSocket API и минимальный
   UI/HA для acceptance-устройств, проверенный браузером.
9. **PR-09 / P0:** `OPS-01..06`: reproducible OpenWrt artifact, место/RAM, rollback,
   HIL и длительная работа; публикуется R1 с перечнем подтверждённых возможностей.

PR — логическая единица; не требование делать огромный коммит. Дробить по
независимым поведению и отрицательным сценариям. Не смешивать перенос storage,
рефакторинг event loop и новые converters в одном изменении.

## Общий контракт исполнения каждой задачи

- Перед кодом: refresh upstream, точный SHA/lockfile, source trace и исходные
  тесты. Указанные ниже источники — стартовые точки, не разрешение угадывать
  непрочитанные ветви.
- Task ID (`STORE-01`, `CV-01` и т.п.) — ID плана. Inventory IDs указаны в
  подразделах; широкий legacy row разбить до начала реализации, сохранив историю.
- Acceptance: одинаковые наблюдаемые данные/ошибки/порядок/retained/QoS,
  persistence и restart; компиляция и HTTP 200 отдельно от parity.
- При изменениях интерфейсов модульная декомпозиция сохраняет `src/lib.rs`,
  thin CLI, один bridge event loop, static `handler_for()`, ZNP actor и 10 s.
- `VERIFIED` только по требованиям AGENTS; HIL только там, где нужен hardware.
  Pure JSON преобразование можно доказать differential без координатора.
- В каждом PR: evidence, inventory, dashboard, тестовые артефакты и release notes
  о оставшихся отличиях. Непройденный gate блокирует соответствующую веху.

Календарный срок сейчас не обоснован: неизвестны real-package footprint Boa,
точные модели Moes/датчика, состояние coordinator backup и доступный RAM.
После A0/A4 оценивать оставшиеся работы по измеренным результатам, а не по
количеству Rust-файлов или строк CSV.
