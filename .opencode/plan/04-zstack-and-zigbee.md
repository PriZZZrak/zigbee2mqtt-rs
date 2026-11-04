# 04. Coordinator, network lifecycle и Zigbee

Inventory: ADAPTER-001/002/003, ZIGBEE-001/002/003, ZCL-001..012,
ADAPTER-004/005. R1 hardware — Dongle-P; CC2531 регрессии обязательны, HIL для
него отложен по решению пользователя, global support сохранён.

## Source map и ограничения

Upstream: herdsman `adapter/z-stack/adapter/zStackAdapter.ts`,
`manager.ts:55–274`, `adapter-backup.ts`, `adapter-nv-memory.ts`, `znp/znp.ts`,
`controller/model/device.ts`, `endpoint.ts`, `controller.ts`, `zspec/zcl/*`,
`zspec/zdo/*`. Tests: `test/adapter/z-stack/{adapter,znp,unpi,structs}.test.ts`,
`test/{controller,device,zcl}.test.ts`.

Сохранить actor transport + oneshot + 10 s, legacy mismatched subsystem SRSP,
library bridge, runtime firmware detection. Нельзя исправлять Dongle-P применением
CC2531 исключений ко всем firmware. USB description не заменяет SYS_VERSION.

## ZB-01 / P0 — startup state machine

1. Зафиксировать firmware profile Dongle-P: SYS_VERSION и capabilities, network
   parameters/NV layout, backup format/version; скрыть ключи в публикуемых traces.
2. Port strategy `resume | restore | commissioning`: read NIB/configured marker,
   active/alternate/preconfigured keys и backup; сравнить с config до мутаций.
3. Покрыть reversed/default extended PAN compatibility, channel changes,
   inconsistent backup, отсутствие NV item, неподдерживаемую backup version.
4. Startup success связывать с actual network state и ожидаемыми events, а не
   фиксированным sleep и warning при nonzero SRSP. Device-info error обрабатывать
   согласно upstream, не объявлять ready с недоказанной coordinator identity.
5. Registration endpoints/GreenPower, adapter options и BDB commissioning
   определить по firmware. CC2531 product_id=0 skips сохраняются.
6. Retry/backoff/startup timeout и cleanup после partial start — отдельные cases.

**Gate:** replay upstream traces для каждого strategy; same-network reboot
Dongle-P не меняет network identity и не требует re-pairing; mismatch не
приводит к молчаливой перепрошивке настроек. Destructive commissioning только на
стенде с отдельным разрешением, не на текущей домашней сети.

## ZB-02 / P0 — request ownership и transport correctness

1. Реальная очередь single-flight SREQ: сейчас `pending` может заменяться новым
   Request. Сохранить один serial actor, не решать проблему вторым event loop.
2. Explicit request lifecycle: queued, sent, completed, timed-out/cancelled;
   поздний SRSP, same command повторно, write failure, serial EOF, cancellation
   future drop. ZNP не несёт request ID — ограничения корреляции документировать.
3. Не терять subsystem 0x00 и mismatch response при ожидаемом cmd1; negative
   tests: чужой cmd1 не завершает request, no pending не создаёт phantom result.
4. Backpressure: incoming bursts не должны блокировать SRSP processing;
   различать control events и reports, измерять overflow и последствия.
5. `rtscts`, serial permissions, unexpected disconnect/reconnect, TCP serial и
   discovery исследовать отдельно; Auto сейчас лишь выбирает ZNP.

**Gate:** deterministic transport actor tests/PTY с fragment/coalesce/FCS/garbage,
timeout 10 s под paused clock, queued cancellation, closed serial, burst; legacy
fixtures сохранены. Reconnect не создаёт дублирующиеся event streams.

## ZB-03 / P0 — ZCL/ZDO transaction service

Это prerequisite реальных `toZigbee`, `configure`, reporting и групп.

- Разделить SREQ→SRSP acceptance, AF_DATA_CONFIRM delivery и device ZCL response.
  Сейчас generic `send_zcl` не доказывает исполнение команды устройством.
- Matcher: IEEE/NWK, endpoint, cluster, TSN, frame type, direction,
  command/default response, manufacturer code; исследовать applicability каждого
  поля и status/error/disableResponse semantics, не добавлять все проверки вслепую.
  Итоговый refresh выявил herdsman `75e47fe` (DRIFT-001): одинаковый command ID
  различается по frame type. Новый upstream regression прочитан; решение о
  backport/следующем baseline принимать отдельно от pinned 10.10.0 oracle.
- Упорядочить retry policies, TSN wrap, route recovery, sleepy devices,
  duplicate/missed messages. Timeouts для ZCL не подменяют transport 10 s.
- Typed operations: read, write, command, readResponse, writeResponse,
  defaultResponse, bind/unbind, configure/read reporting, group operations.
- Возвращать actual response/error host API; не `Promise.resolve()` сразу после
  отправки в канал. Persist attribute cache только по соответствующему событию.

**Gate:** reference bytes и sequences на success, unsupported attribute,
default response failure, wrong endpoint/TSN, timeout и retry. Payload parser
сохраняет unknown attribute ID numeric key, binary Buffer и manufacturer context.

## ZB-04 / P0 — interview и identity

1. Явная модель стадий: announce/address → NodeDesc → ActiveEp → SimpleDesc →
   Basic attributes/quirks → definition selection → configure; точный порядок
   и завершение брать из pinned device interview и app configure extension.
2. Persist known NWK; смена NWK очищает stale secondary index; unknown NWK
   инициирует IEEE_ADDR_REQ с дедупликацией pending requests.
3. Endpoints 1/2/3/242, incomplete descriptors, repeated joins, retry после
   sleeping device; не объявлять interview success по первому SimpleDesc.
4. Type, versions, manufacturerName/ID, modelID, powerSource и endpoint cluster
   sets доступны fingerprint matching. Отсутствующее поле не равно нулю.
5. onEvent/custom read handlers, IAS enrollment, time service и vendor quirks
   инвентаризировать отдельно: они могут быть нужны ещё до конца interview.
6. Remove/leave/rejoin: RF operation, DB/config/state/HA cleanup и exact event
   order; force/blocklist/passlist semantics из reference.

## ZB-05 / P1 — полная ZCL surface

Сохранить нынешние статические handlers. Далее: strings/octet arrays/structs,
signed/unsigned widths, invalid sentinel values, long strings, manufacturer
clusters, command responses, group/broadcast addressing, ZDO network management.
Не выбрасывать Bytes до converter: нынешний enum уже теряет payload.

Fuzz: arbitrary input не вызывает panic/выход за границы; truncated attribute
не превращается в корректный частичный user state без source-backed политики.
Исключения вида unsupported status но successful write — реальные upstream
quirks, например свежий TS011F_5; не «исправлять» их общей проверкой !=0.

## ZB-06 — HIL и расширение

R1: cold start, same-network restart, offline cached runtime, permit join,
interview, Lytko/Moes/sensor reports и commands, coordinator unplug/replug,
broker reconnect и rollback. Записать USB/firmware/Rust SHA/converter SHA.

S1: отдельно CC2531 HA 1.2 с теми же relevant scenarios, backup compatibility,
legacy negative responses; только после реального HIL отмечать hardware-verified.
Другие адаптеры — [09](09-global-parity.md), не сваливать их в ZnpHandle.
