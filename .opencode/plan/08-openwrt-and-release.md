# 08. Routerich/OpenWrt, ресурсы и выпуск

Профиль пользователя: Routerich AX3000 v1, OpenWrt 24.10.8, **54 МБ общего места**,
Dongle-P, Lytko «L101 NS», датчик и Moes. Не считать 54 МБ свободным местом;
RAM, ABI/target и реальный writable overlay необходимо измерить. RPi 3/native
build остаются поддерживаемыми проектными целями.

## OPS-01 / P0 — паспорт стенда

Без изменения сети собрать: `ubus call system board`, `cat /etc/openwrt_release`,
`uname -m`, `cat /proc/meminfo`, `df -k /overlay /tmp /etc`, `opkg print-architecture`,
package versions, mounts и USB serial identifier. Если в установленном образе
менеджер пакетов иной — использовать реально доступный. Не выполнять upgrade.

Device identity брать из existing original `bridge/devices`/database с удалёнными
секретами; coordinator firmware/version — из diagnostics. Network keys не
добавлять в git. Персональная карта устройств содержит count/gang/endpoints,
прошивку, converter identity и ожидаемые controls. TRV пока `NOT_SELECTED`.

**Выход:** source-controlled обезличенный test profile и точные бюджеты,
одобренные владельцем после измерений. Пока budgets UNKNOWN, performance gate
не считается пройденным.

## OPS-02 / P0 — воспроизводимая cross-build/упаковка

1. Pin Rust toolchain/Cargo.lock, OpenWrt SDK version/target и libc ABI. Не
   выбирать aarch64-musl только по маркетинговому названию роутера.
2. CI строит бинарь и package из одного Rust SHA; manifest включает target,
   toolchain, features, binary hash/size и frontend/converter ABI versions.
3. `openwrt/Makefile` сейчас копирует `files/zigbee2mqtt-rs`. Заменить зависимость
   от вручную положенного бинаря воспроизводимым artifact flow; не удалять файл
   до проверки provenance и перехода packaging.
4. Native dependency audit по actual target tree/features/build scripts/
   resulting ELF. `rumqttc default-features=false` сохранить. Разделять pure-Rust
   приложение и системные OpenWrt mosquitto/uhttpd/downloader dependencies.
5. Package содержит service config и разрешения persistent cache; upgrader
   сохраняет database/config/cache. Проверить conffiles/sysupgrade retention.
6. HTTPS download CA store и downloader runtime dependency задекларировать;
   отсутствие curl/wget/uclient-fetch должно быть диагностировано при startup,
   а не обнаружено пустым converter state после join.
7. Windfront assets берутся из pinned build artifact с license/provenance,
   base path совпадает с uhttpd/API route. Remote UI resources учесть для offline.

**Gate:** clean SDK build → install на соответствующий target → start/stop/
upgrade/downgrade/uninstall с сохранением пользовательских данных. Существующий
GNU ARM build в аудит-среде заблокирован отсутствующим target std, не Rust ошибкой.

## OPS-03 / P0 — flash/RAM/CPU budgets

Не загружать полный каталог при старте и не создавать Context на каждый кадр.
Сначала измерить, затем выбирать оптимизацию.

| Измерение | Метод/нагрузка | Условие приёмки |
|---|---|---|
| installed size | бинарь + UI + index + closure + config/db | помещается в измеренный overlay budget |
| update peak disk | old + staging + new + rollback + logs | ENOSPC исключён preflight либо clean failure |
| idle RSS/PSS | работающая домашняя сеть | лимит согласован после OPS-01 |
| cold start peak | registry parse + JS graph + prepared definitions | нет OOM, router services остаются responsive |
| bursts/async jobs | reports + set/get + broker reconnect | bounded queue; latency budget; no silent loss |
| длительная работа | timers, many publishes, repeated updates | heap/RSS выходит на плато, не растёт монотонно |
| flash writes | 1h reports, 100 metadata changes | batching/no-op suppression measured |

Baseline live registry 2 957 412 bytes — лишь JSON на диске, не heap footprint.
Shared transitive graph (особенно Tuya) может быть значительно больше Lytko
65 740 bytes. Hashes проверяют целостность относительно trusted manifest, а не
являются цифровой подписью: прежнее «подписывать sha256» терминологически неверно.

## OPS-04 / P0 — service lifecycle

procd startup после нужных ресурсов, serial ownership, bounded respawn,
остановка по SIGTERM с flush, permissions и path layout. `/etc` persistent
converter location сохранена; не переносить на tmpfs. Logs ротировать с budget.

OpenWrt reboot, broker down при boot, USB slow enumeration, read-only fs,
clock not synchronized и network offline должны иметь определённые исходы.
NixOS service `ProtectSystem=strict` отдельно проверить на совместимость с
жёстким cache path `/etc`; general deployment нельзя считать подтверждённым
одним OpenWrt запуском.

## OPS-05 / P0 — release candidate и rollback

1. Закрыть R1 cases [10](10-acceptance-matrix.md), freeze candidate SHA и package
   versions; не менять latest converters во время HIL под тем же evidence.
2. Экспорт исходной сети в backup, offline round-trip validation. Запустить
   только один bridge на Dongle-P. Переключение со временем окна согласовать.
3. Verify reports, physical Moes switches, Lytko controls и sensor; timezone,
   restart, cache offline, broker reconnect, unplug/replug на controlled стенде.
4. Предлагаемый soak: минимум 72 h с периодическими report/set и одним плановым
   restart; длительность согласовать перед запуском. Это критерий плана,
   не уже полученное свидетельство.
5. Вернуться на original и проверить ту же сеть без pairing, корректные names,
   config/DB и HA IDs. Затем вернуть RC, если все gates проходят.
6. Release содержит checksums, target/profile, baseline/ABI, known gaps,
   migration/rollback инструкции и ссылки на evidence. RC failure не исчезает
   после retry: сохраняется вместе с причиной исправления.

## OPS-06 — лицензирование/операционная поддержка

Upstream app GPL-3.0, converters MIT; Rust manifest сейчас MIT. До дистрибуции
адаптированного app кода/fixtures/assets подтвердить provenance и применимую
лицензию с владельцем, сохранить notices. Не менять LICENSE по предположению.

Далее Docker/systemd/NixOS, ARM GNU и native release CI, diagnostics bundle,
health metrics и upstream delta queue. Никаких неограниченных retention logs,
автоматических destructive migrations или автоустановки непроверенных converters.
