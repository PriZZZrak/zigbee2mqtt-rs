# 11. Решения и открытые вопросы

## Подтверждено пользователем в этом аудите

| Решение | Значение и последствия |
|---|---|
| Первый результат | Перенос существующей сети туда/обратно и реальные операции; приоритет Lytko |
| Платформа R1 | Routerich AX3000 v1, OpenWrt 24.10.8; сообщено 54 МБ общего места |
| Hardware R1 | Только Dongle-P доступен; CC2531 больше не обязательный hardware gate первого релиза |
| Устройства | Lytko «L101 NS», неизвестный датчик температуры, Moes switches; будущие TRV не выбраны |
| Единый механизм | Никаких поблажек для личных устройств; одинаковая обработка всего supported Z2M каталога, личные устройства лишь приоритет разработки/проверки |
| Обновление converters | Проверка полного пакета перед активацией; закреплённая рабочая версия и возможность rollback |

Последнее уточнение пользователя принципиально: R1 нельзя получать отдельной
захардкоженной веткой для Lytko/Moes. Их golden/HIL — первые тесты общей системы.
Vendor-specific logic допустима как часть upstream definition/quirk с теми же
условиями applicability, не как персональный bypass.

Аппаратный scope R1 отделён от Stage1/global метрик. Никаких исключений всего
CC2531 или остальных adapters из global inventory пользователь не одобрял.
Подробная запись решений: `docs/parity/decisions.md`.

## ADR backlog — решить до затрагивающей реализации

### ADR-01. Compatibility baseline и поставляемая версия

App 2.14.2 требует converters 26.115.1 / herdsman 10.10.0 / Windfront 2.15.0.
Pages сейчас 26.125.0, старый tracker Windfront 2.15.1. Рекомендация: восстановить
matched baseline для oracle, отдельно тестировать новый package; следующую
согласованную baseline revision выбрать после dependency/delta review.
**Не решено:** на какой согласованной версии выпускать R1. Нельзя смешать latest
отдельных repositories и объявить набор оригиналом.

### ADR-02. Общий dispatch и native acceleration

Требование единого механизма подтверждено. Рекомендация: semantic authority —
prepared upstream definition. Существующие static Rust handlers сохраняются;
fast-path допускается только с доказанной applicability/equivalence, fallback
для genuinely unknown definitions должен соответствовать отдельному контракту.
**До кода:** показать traces current native vs upstream для стандартного и
vendor-specific cluster, утвердить точную dispatch схему, исключить double calls.
Это не разрешение удалить native handlers или менять ordering без регрессий.

### ADR-03. Boa packaging/context/host API

Выбрать CJS closure или CI bundling, lifetimes contexts, shared store и async
job integration по real-package spike CV-06. Нужны RAM/CPU/flash измерения на
Routerich. Без native code и без отдельного Node процесса на роутере.
Если лимиты не позволяют — вопрос владельцу, а не подмена API пустыми stubs.

### ADR-04. Момент активации и ресурсный бюджет обновлений

Проверка перед активацией и rollback одобрены. **Не определено:** переключать ли
успешно проверенную версию автоматически или только по команде пользователя,
retention старых версий при недостатке места, cadence проверок на роутере.
До решения план предполагает отсутствие самопроизвольного переключения во
время работы сети/HIL. Publication latest не равна activation.

### ADR-05. Durability batching

500 ms coalescing соответствует пожеланию экономить записи. **Не определено:**
какие accepted metadata изменения должны fsync до success response и какой
loss window приемлем при потере питания. Исследовать upstream write:false
callers, затем согласовать поведение; нельзя выдать «batched» за полную
эквивалентность по crash consistency.

### ADR-06. TLS и системные зависимости

Pure-Rust application requirement остаётся. MQTT/HTTP TLS support нужно
исследовать через provider implementation/target tree; ring/aws-lc/native
crypto не добавлять автоматически. Current external downloader использует
системные утилиты OpenWrt — задекларировать dependencies и scope pure-Rust claim.
Silent `mqtts`→plain TCP является дефектом, а не вариантом стратегии.

### ADR-07. Лицензия и поставка frontend

Приложение upstream GPL-3.0, converters MIT, текущий Rust MIT. Provenance
адаптированного кода/fixtures/assets и правила поставки подтвердить с владельцем
до release; не изменять license на основании названия проекта.

### ADR-08. Владение persistent Boa context — выбран вариант 1

**Реализовано:** dedicated `z2m-converters` thread владеет Boa-контекстами;
`ExternalConverter` отправляет owned jobs через `std::sync::mpsc` и ждёт ответ.
Сессия кэшируется по `(IEEE, model)`, module state и `globalThis.__device`
сохраняются, timers выполняются после кадра. Fallback на per-call context при
недоступном actor.

**Ограничения:** async fz Promise не ожидается; timers с будущей задержкой не
планируются самостоятельно; сессии не вытесняются (RAM не измерен); callback
timers не публикует результат; shutdown/panic actor наблюдаются только как
закрытый канал и 10s timeout. Это `IN_PROGRESS`, не parity evidence.

## Необходимые исходные данные (первый шаг OPS-01)

- RAM, реальный writable/free overlay, ABI/architecture/SDK; 54 МБ не считать
  свободными. Установленные версии broker/uhttpd/downloader/CA и logs budget.
- Actual modelID/manufacturerName/endpoints/softwareBuildID Lytko, Moes и sensor;
  количество Moes каналов; версия firmware Dongle-P; наличие coordinator backup.
- Версия оригинала, из которой переносим сеть; используемые config options,
  HA и обязательные для пользователя Windfront действия.
- Согласованное тестовое окно и возможность controlled HIL без риска для
  действующего отопления; ничего не сбрасывать без отдельной команды.
- Модели будущих TRV — после выбора; сейчас не обещать совместимость неизвестных
  аппаратных ревизий. Общий converter pipeline проектируется заранее.

Эти вопросы не останавливают независимые работы по evidence, lossless storage,
publication layout, generic resolver и reference harness. Они блокируют только
соответствующий gate/architecture choice.
