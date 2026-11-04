# 07. Windfront, Home Assistant и общие функции

Inventory: FRONTEND-001/002/003, WINDFRONT-001..026, HA-001/002, GROUPS-001,
OTA-001, MQTT-007..009. R1 — проверенный набор UI/device операций; полный набор
остаётся в S1, а не переносится в adapter stage 2.

## UI-01 — настоящий transport API (P0 для рабочего Windfront)

**Факт:** app `lib/extension/frontend.ts:45–197` реализует HTTP/static + JSON
WebSocket `/api`, token и initial retained/entity snapshots. Windfront
`src/websocket/WebSocketManager.ts` отправляет JSON request и отслеживает
transaction. Это не MQTT wire protocol и не прямое подключение к mosquitto WS.
Original tests `test/extensions/frontend.test.ts:226+` проверяют exact JSON feed.

1. Зафиксировать frontend package: matched app Windfront 2.15.0 или отдельно
   одобренный 2.15.1/новее; записать source SHA/assets hash. Нынешнюю встроенную
   копию assets проверить на происхождение/локальные изменения.
2. Реализовать Rust JSON WS transport `/api` с base_url, token (4401), initial
   snapshots и live events. I/O callbacks передают commands одному bridge loop;
   они не создают второе business state machine.
3. Статические файлы на OpenWrt может обслуживать uhttpd; API направляется к
   Rust endpoint через документированный маршрут. Самостоятельный static server
   для других deployment modes остаётся частью global contract.
4. Message schema/errors, client disconnect, slow client/backpressure, reconnect
   replay, base topic stripping, multiple tabs и state update ordering.
5. Token/same-origin/base path и error payloads исследовать точно; не вводить
   произвольный REST API под видом совместимого `/api`.

**Gate:** неизменённый pinned Windfront подключается в браузере, видит реальные
devices/state/exposes, делает set/get/rename и переживает reconnect. Проверяется
network trace, а не только HTTP 200 для index.html.

## UI-02 — authoritative bridge metadata

`bridge/info`, `devices`, `groups`, `definitions`, extensions/converters,
options schema, config, logging, events и health инвентаризировать по
`lib/types/api.ts` и consumers в UI. Для каждого поля: source, default,
optional/null, обновление, retain и lifecycle.

- Device definition включает prepared exposes/options/endpoint/meta и supported
  capability, не только model/vendor/description из Basic cluster.
- Не выводить support=true по interview_complete. OTA capability и controls
  должны иметь реально исполнимый backing operation.
- Coordinator version/network info берутся из протокола, package/version — из
  manifest; не маскировать Rust версию под upstream compatibility promise.
- UI forms должны принимать ровно advertised writable keys и правильно
  показывать read-only, bounded numeric, composite/list и endpoint features.

## UI-03 — actions с side effects

| Семейство | Сейчас | Требуемая реализация и критерий |
|---|---|---|
| rename/remove/options | частично/пустой ok | STORE+RT: actual action, persist, negative cases, restart |
| restart | log + ok | orderly restart и reconnect UI после него |
| configure/reinterview | частичная инициaция/пустой source | ждать нужного результата, progress/events/failure |
| reporting read/configure | ok без ZCL | реальный ZCL transaction и обновлённый endpoint cache |
| bind/unbind/clear | пустой ok | ZDO операции, group/device target, response/status, persisted binds |
| backup | zip="" | реальный архив всех требуемых файлов + coordinator backup |
| networkmap | nodes + [] links | LQI/routing scans, formats/raw/graphviz по контракту |
| install code/touchlink | пустой ok | validated key/discovery/device operation; HIL по разрешению |
| converter/extension CRUD | пустой ok | storage/load/unload, errors, lifecycle и поддержанные APIs |
| OTA | пустой ok | отдельный pipeline ниже |

Общая приёмка: UI показывает success только при реальном результате; ошибка
operation видна как failure с тем же transaction. Проверять side effect на
координаторе/FS/state, не один response topic. Частичный R1 явно перечисляет
отложенные функции; это не OUT_OF_SCOPE_APPROVED для global inventory.

## HA-PLAN-01 — discovery из определения и lifecycle

Sources: app `extension/homeassistant.ts`, tests `homeassistant.test.ts`,
converters exposes/options, Rust `homeassistant.rs` и MQTT publishing.

1. Составить mapping expose→HA entity для персональных Lytko/Moes/sensor.
   Не объявлять все thermostat modes и диапазон 5..40 только из cluster 0x0201.
2. Проверить IDs/object_ids/topics/availability/device/via_device,
   units/classes/value_templates/command templates, state & command JSON types.
3. Legacy `{ieee}_{type}_{base_topic}`, lowercase IEEE, nested color и
   string ON/OFF сохраняются до source-backed applicability/одобрения изменения.
4. Multiendpoint switch gangs и climate suffixes; HA control→MQTT→actual device.
5. Retained config cleanup при rename/remove/exposes change/disabled setting,
   republish на HA birth/MQTT reconnect, отсутствие дублей после upgrade.
6. Далее полная exposes surface: select/number/text/button/event/update/lock/
   cover/fan и vendor-specific features — перечислить реальные исходные types.

**Gate:** golden full discovery messages плюс broker integration и HA smoke;
клик меняет устройство; rename/restart не плодит entities. HA scopes,
discovery topic и original overrides проверяются по config, не hardcoded.

## FEAT-01 — groups и scenes (полный S1)

- App `extension/groups.ts:186–294`, `publish.ts`, herdsman Group/Endpoint и
  converter scenes — authority. В частности remove_all направлен на endpoint
  устройства, а не очистку одной local group (текущий Rust расходится).
- Zigbee group membership сначала реальная RF operation, затем persistence и
  публикации по source ordering; failure не оставляет фиктивный local membership.
- Group numeric/string IDs, duplicate names, endpoint names, skip_disable_reporting,
  group command fanout/broadcast, group state aggregation и availability.
- Scenes add/store/recall/remove/remove_all/rename, extension fields,
  restart retention, group/device applicability и errors. Нынешний ScenesCluster
  для remote actions не означает готовую scene management subsystem.

## FEAT-02 — OTA (полный S1)

В pinned app OTA вызывает **herdsman device OTA**, это не только JS converters.
Прочитать `extension/otaUpdate.ts`, herdsman `device-ota.test.ts`, model/device
и actual OTA helpers/index sources; выяснить resolved package dependencies.

1. Metadata/sources/index override, availability check throttling, версия,
   hardware/manufacturer/image type, checksums и parsing bounds.
2. HTTP retrieval/cache/size budget под роутер, streaming если соответствует
   parser needs; повторный запрос и broken download не забивают overlay.
3. QueryNextImage/ImageBlock/Page/UpgradeEnd, TSN, block size/delay,
   timeout/retry/status и progress/remaining публикации.
4. Schedule/unschedule/downgrade/abort, simultaneous updates policy, sleeping
   device, restart clears stale progress согласно reference.
5. Сначала recorded image/frames и fake device; hardware upgrade только после
   explicit approval конкретного теста и recovery procedure.

**Gate:** protocol transcript + published update state совпадают на success,
invalid image, no update, cancellation, retry и restart; пустой ok не case.

## FEAT-03 — remaining global application surfaces

External converters и external extensions — разные scopes. Extension API
использует app services/events/fs, не ограничивается Fz/Tz host API. Отдельный
design, external override order, unload/reload/stop, errors и runtime permissions
ровно по исследованному контракту. Также onboarding, health, logging settings,
custom icons/static routes, backup/restore tooling и frontend package selection
остаются в shared Stage 1. Планировать API inventory rows до реализации.
