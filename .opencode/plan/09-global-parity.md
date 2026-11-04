# 09. После R1: полный S1 и глобальная совместимость

R1 не исключает ни одно upstream поведение. Global inventory и Stage1 denominator
продолжают расти при обнаружении пропусков. Верификация считается по source
snapshot, отдельные hardware варианты отражаются независимо.

## S1 closure: семь направлений

1. **Configuration/runtime/operations:** полный schema/defaults/migrations,
   env/secrets, startup/shutdown/crash recovery, CLI paths, logging, backups,
   deployment modes и downgrade contracts.
2. **MQTT/state/integrations:** все topic forms, request errors/transactions,
   auth/TLS/QoS/retain, availability/cache/timing, HA entities/cleanup и все
   предусмотренные оригиналом интеграции.
3. **Z-Stack:** вся публичная adapter/network surface, backup/restore/NV versions,
   discovery/TCP, route failures, install codes, reconnect, CC2531 HA1.2 и
   CC2652P firmware matrix. CC2531 HIL не блокирует R1, но отсутствие evidence
   не позволяет объявлять hardware parity этого варианта.
4. **Zigbee protocol/network:** весь ZCL/ZDO parsing/serialization, endpoint/group/
   bind/reporting, IAS/GreenPower/time/quirks, malformed-input robustness.
5. **Catalog/converters:** все baseline definitions с проверяемыми exposes,
   from/to/configure/onEvent/options, external overrides и vendor quirks;
   metadata enumeration не является execution coverage.
6. **Groups/scenes/OTA:** полноценные protocol/persistence/error workflows,
   включая schedule/abort/restart; не заглушки UI.
7. **Frontend/API/extensions:** полный matched package/API contract, auth,
   static serving, device icons, configuration/UI flows, external extension
   lifecycle и services.

Каждое направление разбить на rows/cases до реализации. Для закрытия S1 нужны
инвентаризационный аудит всех upstream domains и отсутствие скрытого backlog,
а не просто все строки неполного CSV помеченные VERIFIED.

## Adapter stages

Upstream `src/adapter/adapter.ts:76–115` реально выбирает:
`zstack`, `ember`, `deconz`, `zigate`, `zboss`, `zoh`, **deprecated `ezsp`**.
Deprecated не значит исключён. Ember и EZSP — разные inventory families.

### ADAPT-PLAN-01 — общий boundary после стабильного Z-Stack

- Убрать публичную зависимость CoordinatorHandle.inner от конкретного
  ZnpHandle через минимальный library adapter interface после source audit.
  Не менять event ordering/one-owner business topology.
- Capability matrix: start/stop, version/network params, ZDO/ZCL,
  backup/restore/install-code/touchlink, transport selection/discovery,
  waiter/cancel/error behavior. Unsupported capability соответствует upstream
  конкретного adapter, а не «все поддерживают всё».
- Contract suite общая, wire traces adapter-specific; повторить Z-Stack
  regressions перед добавлением нового backend.

### ADAPT-PLAN-02 — план каждой семьи

| Семья | Источник | Работа перед кодом | Аппаратный gate |
|---|---|---|---|
| ember | `adapter/ember/adapter/emberAdapter.ts` + transport/stack | Ember API/versions, ASH transport, security/backup, NCP responses | выбрать реально доступный Ember coordinator, не Dongle-P |
| ezsp | `adapter/ezsp/adapter/ezspAdapter.ts` и зависимости | legacy EZSP versions и отдельная карта divergences | legacy NCP/firmware, не считать Ember traces заменой |
| deconz | `adapter/deconz/adapter/deconzAdapter.ts` | serial queue/protocol, endpoint/network/backup applicability | конкретный ConBee/RaspBee вариант |
| zigate | `adapter/zigate/adapter/zigateAdapter.ts` | framing/commands, network restore, retries | согласованный ZiGate firmware |
| zboss | `adapter/zboss/adapter/zbossAdapter.ts` | framing/ACK/window, startup/NVRAM, responses | supported ZBOSS NCP |
| zoh | `adapter/zoh/adapter/zohAdapter.ts` + resolved `zigbee-on-host` | большая stack dependency, pure-Rust feasibility, host resources | hardware/transport по pinned implementation |

Для **каждой** семьи отдельные подзадачи:
1. Pin exact dependency; прочитать transport + adapter + controller callers +
   original tests; перечислить quirks/versions.
2. Frame codec/property tests, command correlation/timeouts/cancellation.
3. Startup/resume/restore state machine и network parameters.
4. Join/interview/leave, report/read/write/command, group/bind/reporting.
5. Disconnect/recovery/shutdown/backup и malformed frames.
6. Shared MQTT/HA/frontend/storage differential suite поверх нового backend.
7. HIL реального варианта, documented limitations, обновление inventory.

Порядок семей после Z-Stack определить по доступному hardware и пользовательской
потребности. Сейчас оснований назначать даты/обещать ZoH на 54 МБ нет.
Если dependency требует native код — BLOCKED/исследование pure-Rust port,
не молчаливое нарушение AGENTS и не удаление family из global target.

## Catalog maintenance

Сохранять baseline catalog manifest + delta manifest new/changed/removed
definitions, converters и exposed options. Новая upstream прошивка/модель
открывает соответствующие cases, не автоматически все 5000 devices VERIFIED.

Differential suites группировать по converter behavior и дополнительно по
definition-specific configuration; common function test не заменяет каждый
vendor fingerprint/configure quirk. HIL выборка обозначается как выборка.
Полное hardware coverage всех каталогов не обещать без соответствующей метрики.

## Baseline advancement и поддержка

Daily fetch/report → triage commits → source/test analysis → новые cases →
reviewed candidate baseline → reference capture/differential → migration notes
и coordinated dependency pins. Latest publication и approved runtime activation
разделены. Вновь затронутые VERIFIED rows переоткрываются с причиной.

Глобальный release gate: audited inventory complete, все обязательные contracts
подтверждены против точного baseline, drift и exclusions честно перечислены,
все relevant negative/restart/hardware gates пройдены. «100%» без denominator
и scope не использовать даже после закрытия roadmap.
