# 10. Матрица приёмки и доказательства

Все строки ниже — **запланированные сценарии**, сейчас не PASS. Обозначения:
R = pinned reference execution, D = differential Rust/Node, I = integration,
H = hardware-in-loop, O = OpenWrt эксплуатационная проверка. Synthetic fixtures
проверяют инфраструктуру, но сами по себе не доказывают upstream compatibility.

У каждого запуска: ID, Rust SHA, baseline SHA/package integrity, входные fixtures,
команда/среда, ожидаемый observable результат, actual artifact, verdict и причина.
Секреты заменять только согласованными fixture значениями; не убирать различия
в именах, endpoints, errors, QoS или последовательности событий.

## A. Обязательные R1 gates

| Case ID | Предусловие/стимул | Проверяемый результат | Evidence | План |
|---|---|---|---|---|
| R1-01 | upstream config со всеми секциями; rename одного устройства | остальные поля, references и options сохраняются при чтении оригиналом | R/D | STORE-01 |
| R1-02 | devices/groups в нескольких YAML, secret refs, env overrides | precedence/write-back совпадают; повторная запись идемпотентна | R/D | STORE-01 |
| R1-03 | upstream DB, 100 sync и restart | ID, endpoints/epList, metadata, binds/reporting и groupID стабильны | R/D | STORE-02 |
| R1-04 | новый device и group, sync/restart/remove/rejoin | устойчивые ID; нет потерянных или дублированных records | D/I | STORE-02 |
| R1-05 | ENOSPC/read-only/прерывание между write/fsync/rename | сохранён корректный основной файл, ошибка видна, dirty revision не потеряна | I/O | STORE-03 |
| R1-06 | SIGTERM в debounce window | accepted metadata сохранена после рестарта | I/O | STORE-03/RT-02 |
| R1-07 | работающая сеть Dongle-P, те же config и DB | resume без commissioning/изменения identity/re-pairing | R/D/H | ZB-01 |
| R1-08 | mismatch config/NV/backup на fixture-стенде | source-backed error/strategy до опасных write; нет ложного ready | R/D | ZB-01 |
| R1-09 | SRSP subsystem 0x00/mismatch, wrong cmd1, late response | legacy delivery сохранён; чужой response не завершает операцию | D/I | ZB-02 |
| R1-10 | serial EOF/reopen и очереди под нагрузкой | recovery/выход по политике; ни duplicate stream, ни зависшие requests | I/H/O | ZB-02/RT-02 |
| R1-11 | неизвестный NWK и смена NWK известного IEEE | IEEE lookup, обновление индекса, subsequent command правильному устройству | D/H | ZB-04 |
| R1-12 | join/interview с incomplete/repeated descriptors | корректные этапы/ошибки; endpoint identity доступна resolver | D/H | ZB-04 |
| R1-13 | чистый cache, published package download | весь граф модулей доступен по layout manifest, нет checkout fallback | I/O | CV-01..04 |
| R1-14 | modelID != product model, null bucket, prioritized fingerprints | выбран тот же definition/whiteLabel, что Node | R/D | CV-03/05 |
| R1-15 | real compiled module с modernExtend и bare imports | подготовленное definition совпадает; нет undefined fake API | R/D | CV-05..07 |
| R1-16 | report с реальным endpoint/LQI/manufacturer | совпадают named/binary data и MQTT result, ничего не заменено endpoint=1 | D/H | CV-07/08 |
| R1-17 | Promise fz + publish callback + timer + второй кадр | порядок публикаций, retained state/store и errors совпадают | R/D/I | CV-07/08 |
| R1-18 | MQTT get с несколькими ключами | реальные read/convertGet, ответы, ошибки; не выдача cache вместо read | D/H | RT-04/CV-08 |
| R1-19 | MQTT set success/failure/timeout/optimistic=false | actual command и conditional state; нет false success | D/H | ZB-03/CV-08 |
| R1-20 | native-covered cluster с device-specific converter | generic механизм сохраняет все upstream quirks без double publish | R/D/H | CV-08 |
| R1-21 | Lytko пользователя: все exposed R1 controls/reports | правильные endpoints/units/settings и restart persistence | R/D/H | CV-09 |
| R1-22 | Moes пользователя: все gangs и физические переключения | правильные identity/endpoints/actions, command и report согласованы | R/D/H | CV-09 |
| R1-23 | датчик пользователя: signed/invalid/battery/sleep | payload/timing/availability соответствуют definition и options | R/D/H | CV-09 |
| R1-24 | дополнительная не пользовательская definition fixture | тот же pipeline работает без whitelist Lytko/Moes/персональных веток | R/D | CV-05..09 |
| R1-25 | обновление прерывается, повреждён digest/ABI, мало места | active last-known-good не меняется; update/rollback атомарны | I/O | CV-04 |
| R1-26 | restart при отсутствии Интернета с прогретым cache | существующие устройства работают на закреплённой версии | I/H/O | CV-04 |
| R1-27 | broker restart/auth failure, поток reports/commands | subscribe/online/offline/republish без потери контекста и ложного online | R/D/I | RT-02/03 |
| R1-28 | rename/remove/options и restart | actual side effects, retained cleanup, config/DB и responses согласованы | R/D/I/H | RT-05 |
| R1-29 | malformed/unsupported request, ошибка ZCL/storage | source-backed error; не пустой status=ok | R/D/I | RT-05/UI-03 |
| R1-30 | неизменённый Windfront, connect/reconnect/token | JSON /api snapshots/events/actions; без подмены на MQTT WS | R/I/O | UI-01/02 |
| R1-31 | UI управления выбранными acceptance devices | реальные set/get, endpoints/exposes и отображение failures | I/H/O | UI-03 |
| R1-32 | HA discovery/control/rename/restart | IDs/templates/payloads и cleanup, без duplicate entities | R/D/I/H | HA-PLAN-01 |
| R1-33 | package clean build/install/upgrade | artifact SHA/ABI известны, config/DB/cache сохраняются | I/O | OPS-02/04 |
| R1-34 | cold start + burst + download + update на роутере | согласованные RAM/disk/latency бюджеты, без OOM/overlay exhaustion | O | OPS-03 |
| R1-35 | длительная работа по согласованному soak schedule | no leak/unbounded queue, recovery подтверждён traces | H/O | OPS-05 |
| R1-36 | original → Rust → original на том же Dongle-P | сеть/данные/устройства переносимы без повторного pairing | R/H/O | STORE-04/OPS-05 |

«Все controls R1» определяются из **фактического** prepared definition и полного
списка требований, до тестов. Нельзя оставить неудобные поля за пределами списка
после failure. Если для R1 сознательно откладывается возможность, требуется
явный release gap и решение пользователя; общий каталог не уменьшается.

## B. Дополнительные gates полного Stage 1

| Набор | Сценарии |
|---|---|
| S1-LEGACY | CC2531 HA1.2 startup/restart/join/report/commands/recovery/persistence HIL; legacy SRSP cases |
| S1-CONFIG | все schema properties/defaults/validation/migrations/env/ref combinations по audited inventory |
| S1-MQTT | все topics/output modes/endpoint routes/QoS/retain/auth/TLS и отрицательные случаи |
| S1-STATE | cache persistence, availability active/passive/groups, throttle/debounce/timer teardown |
| S1-HA | полный exposes→discovery mapping, migration/cleanup/overrides/HA birth |
| S1-GROUP | Zigbee membership, group commands/state, bind/unbind/reporting, scenes persistence |
| S1-OTA | check/download/protocol/progress/schedule/abort/errors/restart; согласованный HIL |
| S1-UI | каждый endpoint и UI flow, auth/static routes/icons/backup/networkmap/extensions |
| S1-CATALOG | исчерпывающий catalog audit, per-definition/converter cases, external override precedence |
| S1-OPS | все обещанные deployment/backup/restore/reconnect/logging/security contracts |

Наборы B ещё нужно разбить на independently testable inventory rows. Таблица не
задаёт выдуманное total_cases=10. Для G повторить relevant shared suites поверх
каждого adapter backend и добавить его protocol/firmware tests.

## Артефакты и release checklist

- Node/Rust command logs и verdict machine-readable, с nonzero exit на mismatch.
- Raw inputs, expected reference outputs, actual outputs; `.db`/YAML snapshots;
  брокерные retain/QoS; ordering graph/clock tolerance; resource measurements.
- Device/hardware profile обезличен; secrets excluded; trace reproducible.
- Failure→fix→rerun history сохранён. Нельзя заменить failure новым stdout
  успешного unit test, который не исполняет тот же путь.
- R1 gate не требует CC2531 HIL по новому решению пользователя; остальные
  historical hardware claims остаются неподтверждёнными.
- Перед release refresh upstream и per-behavior drift review; baseline freeze
  во время HIL; dashboard и known gaps соответствуют фактическому artifact.
