# План: веб-интерфейс для zigbee2mqtt-rs (без LuCI, без TCP-адаптеров)

Решение пользователя: LuCI не нужен, TCP/внешние адаптеры — не сейчас.
Остается только веб-UI. Всё делается в рамках существующего OpenWRT-пакета.

## 0. Ключевое наблюдение

MQTT-интерфейс бриджа уже совместим с оригинальным zigbee2mqtt
(bridge/state, bridge/devices, bridge/info, bridge/logging, `*/set`, `*/get`,
HA discovery). Поэтому **оригинальный фронтенд zigbee2mqtt можно использовать
без изменений в Rust-коде** — достаточно отдать его статику и включить
WebSocket в mosquitto. Никаких изменений в бинаре не требуется.

## 1. Что делаем

### Шаг 1. WebSocket в mosquitto

Добавить листенер в конфиг mosquitto (в пакет, `/etc/mosquitto/conf.d/99-zigbee2mqtt.conf`):

```
listener 9001
protocol websockets
```

Порт 9001 — стандартный для z2m-фронтенда. Если включена MQTT-аутентификация,
фронтенд её поддерживает (вводится в UI, хранится в localStorage).

### Шаг 2. Получить готовую сборку фронтенда (без Node.js)

Node.js в окружении нет, поэтому сборку из исходников не делаем.
Берём готовый релиз `zigbee2mqtt-windfront` с GitHub Releases:

```sh
# скачать последний релиз (архив с dist/)
wget https://github.com/Nerivec/zigbee2mqtt-windfront/releases/latest/download/...
# распаковать в frontend/dist/
```

Артефакт: `windfront/dist/` (~2-3 МБ минифицированных `index.html`, `*.js`, `*.css`).

Опционально — сгенерировать `settings.json` с дефолтами (base_topic из конфига),
чтобы пользователю не вводить параметры подключения руками.

### Шаг 3. Включить фронтенд в ipk

В `openwrt/Makefile` добавить установку файлов в `/www/zigbee2mqtt/`:

```
define Package/zigbee2mqtt-rs/install
    ...
    $(INSTALL_DIR) $(1)/www/zigbee2mqtt
    $(CP) $(PKG_BUILD_DIR)/frontend/dist/* $(1)/www/zigbee2mqtt/
endef
```

uhttpd уже раздает `/www`, так что фронтенд будет доступен по
`http://<router-ip>/zigbee2mqtt/`.

### Шаг 4. postinst — настройка mosquitto

В postinst-скрипте пакета:
- создать `/etc/mosquitto/conf.d/99-zigbee2mqtt.conf` с WS-листенером;
- перезапустить mosquitto (`/etc/init.d/mosquitto restart`).

### Шаг 5. Документация

README: раздел «Web UI» — URL `http://<router>/zigbee2mqtt/`, порт WS 9001,
параметры подключения (broker URL `ws://<router>:9001`, base_topic `zigbee2mqtt`).

## 2. Что НЕ делаем

- Никаких изменений в Rust-бинаре (интерфейс уже совместим).
- Никакого LuCI-приложения и UCI-обёртки.
- Никаких TCP-адаптеров и ezsp (отложено).
- Не делаем собственный веб-сервер в Rust — uhttpd уже есть.

## 3. Порядок работ

1. Шаг 1 (WS в mosquitto) — конфиг, проверить на роутере.
2. Шаг 2 (сборка фронтенда) — локально, убедиться что `dist/` собирается.
3. Шаг 3 (ipk с фронтендом) — обновить Makefile, пересобрать ipk, проверить размер.
4. Шаг 4 (postinst) + Шаг 5 (документация).

## 4. Риски

- **Размер flash.** Фронтенд ~2-3 МБ + бинарь 1.7 МБ = ~4.5 МБ. Для 8 МБ-роутеров
  на грани. Вариант: вынести фронтенд в отдельный пакет `zigbee2mqtt-windfront`,
  который можно не ставить (тогда UI доступен только по MQTT/HA).
- **Сборка фронтенда.** Нужен Node.js в CI. Если не хочется — можно тянуть
  готовый релиз `zigbee2mqtt-windfront` с GitHub releases вместо сборки.
- **Auth.** Если MQTT с паролем — пользователь вводит credentials в UI при
  первом заходе (фронтенд это умеет).

## 5. Ближайший шаг

Шаг 1: добавить WS-листенер в mosquitto на роутере и проверить, что
фронтенд (если поставить вручную в `/www/zigbee2mqtt/`) подключается.
После этого — Шаги 2-4 в одном PR.
