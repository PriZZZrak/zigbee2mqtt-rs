# Архив прежнего плана converters (не действующий план)

**Отозванные утверждения о готовности.** Содержимое ниже сохранено как история;
DONE/90%, размеры, версии, схемы публикации и описания реализации в нём не
считать текущими фактами. Аудит выявил существенные расхождения. Актуальный план:
[05-converters.md](05-converters.md), оценка: [01-current-state.md](01-current-state.md).

## Историческая цель — ошибочно объявлялась DONE 2026-11-02
Поддержка 4000+ устройств без файла 5MB в репо, 90% windfront, pure Rust (`boa_engine` без C), RPi3 aarch64. Реализовано: GH Actions + `src/converters/*` common libs.

## Baseline
- upstream: `.upstream/zigbee-herdsman-converters:9dabbe4` v26.115.1, `src/devices/*.ts`, `src/converters/*`
- Rust: `src/zigbee/zcl/clusters/mod.rs:37` handler_for, `src/homeassistant.rs`, `src/converters/` отсутствует

## Решение (on-demand, как предложил пользователь)

### 1. GH Actions конвертация (репо PriZZZrak, не в бинаре)
- Файл: `.github/workflows/convert-converters.yml`
- Триггер: `on: release` в `Koenkk/zigbee-herdsman-converters` (отслеживание релизов) + `schedule` еженедельно для проверки + `workflow_dispatch` с `converters_ref` (default 9dabbe4). Использовать `repository_dispatch` или `actions/github-script` для опроса `https://api.github.com/repos/Koenkk/zigbee-herdsman-converters/releases/latest` и сравнения `tag_name` с последним опубликованным в `gh-pages`
- Шаги:
  1. `actions/checkout` `.upstream/zigbee-herdsman-converters` @ `converters_ref` (tag релиза, напр. `v26.115.1`)
  2. `npm ci && npm run build` → `dist/` JS (уже есть, TS→JS)
  3. Скрипт `scripts/gen-registry.mjs` (Node, pure): парсит `dist/devices/*.js` → генерит `registry.json` ~100KB `{ "IKEA/TRADFRI bulb E27": "ikea/tradfri.js", "Lytko/L101Ze": "lytko/l101ze.js", ... }` + копирует `dist/converters/*.js`
  4. Публикует в `gh-pages` ветку `herdsman-converters/v26.115.1/` — без коммита в `main`, каждый релиз — новая папка, `latest` симлинк → `v26.115.1`
- Артефакт: `https://prizzzrak.github.io/zigbee2mqtt-rs/herdsman-converters/v26.115.1/registry.json` + `.../*.js` ( `latest/registry.json` → 302 )

### 2. Рoутер-бинарь — историческое неподтверждённое DONE
- `src/converters/external.rs` (новый, единственный JS рантайм):
  - Зависимость `boa_engine:0.19` (pure Rust, `cargo tree` без `ring`/`aws-lc`, проверено)
  - При старте `mkdir -p /etc/zigbee2mqtt-rs/herdsman-converters` (файлы маленькие ~5-20KB, persist в `/etc`)
  - Кэш `registry.json` (фетч раз в 24h, `If-None-Match`, `latest/registry.json` → конкретная версия)
  - На `CoordinatorEvent::DeviceJoined` / `SimpleDescRsp` с неизвестным `modelID` ( `src/bridge.rs:116`, `src/devices/mod.rs:13` ) → lookup `vendor/model` в `registry.json` → если найден → `GET .../v26.115.1/lytko/l101ze.js` → сохраняет в `/etc/zigbee2mqtt-rs/herdsman-converters/lytko/l101ze.js` → `boa_engine` `Context::eval` → вызов `fromZigbee`/`toZigbee` (stub `eval` сейчас, дальше `rquickjs`→`boa`)
- Core 20 кластеров в `src/zigbee/zcl/clusters/` остаются Rust (`handler_for` static), не дублируются в JS — JS только для device-specific `fromZigbee`/`toZigbee` вне кластеров
- `src/bridge.rs:DeviceJoined` — сначала пробует Rust `handler_for`, затем `external.rs` fallback
- LRU чистка: `herdsman-converters/` не растёт безлимитно — `max 50 файлов` или `5MB`, удалять oldest `atime`

### 3. Конфигурация
- `config.example.yaml` → `converters_registry: https://prizzzrak.github.io/zigbee2mqtt-rs/herdsman-converters/latest/registry.json` (по умолчанию `gh-pages` + отслеживание релизов), `converters_dir: /etc/zigbee2mqtt-rs/herdsman-converters`
- `Cargo.toml` — без C, `rumqttc default-features=false` сохранён

## Альтернативы отклонены
- Один файл `generated/definitions.rs` 5MB — медленно компилируется, нечитаемо
- Полный рантайм `rquickjs` для всех 4000 устройств offline — +2MB к бинарю, RAM на RPi3

## Риски / проверки
- Offline pairing без сети → fallback `Unknown` (как в `docs/parity/evidence/ZIGBEE-join-interview.md` quirks) — приемлемо для 90% windfront
- Безопасность — `registry.json` подписывать `sha256`, проверять перед `evaluate`
- Flash — `/etc` 4.5MB уже (фронтенд `openwrt/files/windfront`), JS по требованию ~5-20KB/устройство, чистить LRU

## Метрики (inventory.csv)
- Добавить `CONVERTER-001..00N` stage 1 для 10 топ-устройств (IKEA, Xiaomi, Lytko) — `VERIFIED` после `build.rs` golden, остальные `DISCOVERED` stage 2
- `ADAPTER-001/002` остаются `PARTIAL` до HIL, `CONVERTER` не завышает `Stage1` — раздельные `device parity` vs `behavior parity`

## Этапы
1. GH Actions + `registry.json` (1 PR, нет Rust)
2. `src/converters/external.rs` + `rquickjs` + тест на Lytko L101Ze (`ref/L101Ze_z2m.js`)
3. Интеграция в `src/bridge.rs` + `docs/parity/evidence/CONVERTER-external.md`

## Решение: /etc + gh-pages
- Хранение: `/etc/zigbee2mqtt-rs/herdsman-converters` (persist, файлы маленькие) — как ты выбрал
- Публикация: `gh-pages` с отслеживанием релизов оригинала (не Release asset) — каждый новый релиз `Koenkk/zigbee-herdsman-converters` → новая папка `vX.Y.Z` + `latest` симлинк
