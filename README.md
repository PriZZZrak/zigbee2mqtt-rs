# zigbee2mqtt-rs

Чистый Rust-мост Zigbee → MQTT для embedded ARM / OpenWrt, цель — заменить
Zigbee2MQTT без изменения MQTT-потребителей, сущностей Home Assistant и
операционных ожиданий.

> **Статус: чистый старт (2026-11-06).** В `main` начинается новая реализация.
> Первая Rust-попытка отправлена в архив. Тестовый роутер сейчас работает на
> оригинальном Node.js Zigbee2MQTT; Rust-бинарник на нём не развёрнут.

## Решение по R1

R1 — это **milestone ядра**, а не каталога устройств:

- ZNP / координатор и Zigbee stack;
- MQTT bridge API;
- Home Assistant discovery;
- HTTP/WebSocket backend для Windfront;
- хранение, state cache, импорт/миграции и операции.

R1 сознательно исключает `zigbee-herdsman-converters`, JavaScript-движки
(Boa), vendor-specific quirk’и и полный каталог устройств. Ядро должно быть
**полностью рабочим** — без заглушек, неограниченных ожиданий и дедлоков.

- Железо: Routerich AX3000 v1, OpenWrt 24.10.8 (aarch64), SONOFF Dongle-P
  (TI CC2652P / Z-Stack).
- Ограничение: **pure Rust, без C-зависимостей**.
- Разработка ведётся напрямую в `main`.

## Что случилось с v1

Первая реализация встраивала **Boa** для исполнения upstream-пакета
конвертеров. На x86 она работала, на 2-ядерном роутере — нет:

- модули конвертеров переоценивались на каждое устройство/сессию, а не один
  раз на пакет;
- генерация определений/exposes ждалась event loop’ом моста;
- process-wide Boa-lock и запись БД могли блокировать MQTT/фронтенд на
  десятки секунд и приводить к дедлоку после interview/удаления.

Node не виснет здесь потому, что `require` кэширует модули, V8/JIT исполняет
их один раз, поиск определений идёт в памяти, а I/O асинхронный. Архив
сохранён как доказательство:

- ветка: [`archive/boa-js-host-20261106`](https://github.com/PriZZZrak/zigbee2mqtt-rs/tree/archive/boa-js-host-20261106) (коммит `502601f`);
- тег: `archive-boa-js-host-20261106`;
- внутри: предыдущий код, parity inventory, HIL-доказательства, OpenWrt-пакет
  и исходный `AGENTS.md`.

Не копируй converter-host из v1. Правила для переписывания — в
[`AGENTS.md`](AGENTS.md).

## Объём R1

«100% рабочее» означает, что следующие поверхности завершены:

- **ZNP/координатор**: serial transport, ZNP framing, сопоставление
  SREQ/SRSP, старт/резюм без перезаписи NV, параметры сети, permit join,
  reconnect, определение возможностей прошивки.
- **Zigbee stack**: ZDO announce/address/descriptors/interview/bind; ZCL
  read/write/report/configure-reporting, default responses, группы,
  кодеры/декодеры стандартных кластеров. Generic state/commands устройств —
  **pending Q1**.
- **MQTT**: `bridge/state`, `info`, `devices`, `converters`, `groups`,
  `health`, `logging`; `bridge/request/*` для `options`, `permit_join`,
  `device/rename`, `device/remove` (leave + block), `device/interview`,
  `health_check`, `networkmap`, `restart`; device `set`/`get`; транзакции,
  retained/QoS, reconnect.
- **Состояние/хранение**: round-trip `database.db` NDJSON,
  `configuration.yaml`, `state.json`, optimistic state, availability,
  last seen; восстановление после restart, kill/respawn и reboot.
- **Home Assistant**: bridge entities, device-registry discovery, очистка при
  rename/remove. Per-device cluster discovery — **pending Q1**.
- **Frontend**: немодифицированный Windfront через HTTP/WebSocket, включая
  ответы bridge, replay состояния и диспетчеризацию команд через
  WebSocket-шину.
- **Операции**: CLI, логирование, сервис OpenWrt, сборка aarch64 musl,
  инструменты импорта/миграции, rollback-safe persistence.

### Вне R1

- `zigbee-herdsman-converters`, Boa/JS, vendor-specific exposes/quirk’и.
- Полный каталог устройств и per-model конфигурация.
- Адаптеры кроме Z-Stack (ember, deconz, zigate, zboss, zoh, ezsp).
- Приёмка CC2531 (R1 использует Dongle-P; CC2531 остаётся обязательным для
  полного Stage 1 позже).
- OTA-прошивка, сцены, полный функционал Windfront, проверка live HA.

## Решения и открытые вопросы

Зафиксировано (2026-11-06):

- **Q2:** для приёмки R1 достаточно HIL только с координатором Dongle-P;
  реальное устройство не требуется.
- **Q3:** список поверхностей MQTT/HA/Windfront/хранения выше — это и есть
  определение «100% рабочее» для R1.

Открыто:

- **Q1:** входит ли в R1 generic state/commands стандартных ZCL-кластеров для
  произвольных устройств, или слой устройств полностью вне R1? Пользователь
  пока не определился. Device state/commands и per-device discovery остаются
  заблокированными до ответа; остальное ядро можно делать.

Отслеживается в [`AGENTS.md`](AGENTS.md) §5.

## Работа с архивом

```bash
# посмотреть без переключения
git show archive/boa-js-host-20261106:AGENTS.md
git show archive/boa-js-host-20261106:docs/parity/README.md

# переключиться на архивную реализацию
git checkout archive/boa-js-host-20261106

# вернуться в main
git checkout main
```

На роутере сейчас Node.js-версия. Для тестов Rust на железе нужны явная
пересборка, redeploy и согласование.

## Принципы разработки

Полные правила: [`AGENTS.md`](AGENTS.md). Кратко:

- измеряй на реальном роутере до выбора архитектуры;
- никакого JS/интерпретатора конвертеров в мосту;
- никакой повторной оценки модулей на устройство;
- никакого process-wide lock, которого ждёт event loop;
- ограничивай любое ожидание; ответ — до тяжёлой работы;
- один event loop моста; тяжёлая работа — через worker’ы/каналы;
- никаких заявлений о parity без исходников, тестов и железных доказательств.

## Сборка (когда появится код)

```bash
cargo build --release
cargo build --release --target aarch64-unknown-linux-musl
cargo test
```

Ожидаемый бинарник для роутера — статический aarch64 musl. Нативные/C-
зависимости не допускаются.

## Лицензия

В архивной реализации учитывались GPL-3.0-последствия upstream-экосистемы.
Лицензирование и атрибуция для переписывания должны быть подтверждены до
распространения бинарников или скопированных fixtures.
