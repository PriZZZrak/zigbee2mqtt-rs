# 02. Baseline, учёт и проверяемость

Inventory: RUNTIME-001, CONFIG-001/002, CONVERTER-006, FRONTEND-001,
ADAPTER-001..005, ADAPTER-010..015, CONVERTER-007..012, FRONTEND-002/003,
DRIFT-001..003. Audit evidence: AUDIT-20261104. Приоритет P0.

## EVID-01 — восстановить воспроизводимый baseline

**Вход:** `upstream-lock.json`, app `package.json` + `pnpm-lock.yaml`, исходные
checkouts. App 2.14.2 закреплён на `344b6e03...`; herdsman 10.10.0 и converters
26.115.1 совпадают с app. App реально требует Windfront **2.15.0**, тогда как
старый lock фиксирует **2.15.1**. Отдельно app содержит classic frontend 0.9.21.

Шаги:
1. Разделить compatibility baseline, currently distributed packages и latest
   upstream. Pages v26.125.0 не заменяет baseline converters 26.115.1.
2. Записать все package integrities, git tag→SHA, package manager, Node engine,
   hash lockfile, source/package correspondence. Проверить вложенный herdsman,
   фактически resolved в converters release; не подставлять latest herdsman.
3. Для Windfront установить matched reference 2.15.0 в отдельном read-only
   snapshot; 2.15.1 хранить как дополнительную исследованную версию. Выбор нового
   compatibility baseline — отдельное решение, не «исправление цифры».
4. AGENTS upstream местами устарели относительно package.json; версии брать
   из pinned manifest/lock, конфликт фиксировать. Не применять жёстко pnpm 10
   ко всем последующим upstream tags.
5. Повторить capture в изолированном каталоге, не устанавливая node_modules
   и build output в reference checkouts.

**Gate:** из чистой CI среды устанавливаются строго те же зависимости; provenance
связывает каждую fixture и artifact с SHA/version/integrity; mismatches блокируют
parity, а не автоматически меняют baseline.

## EVID-02 — привести inventory к доказуемому состоянию

1. Исправить 31 смещённую строку без потери notes; запретить неверные enum/columns.
2. Проверить 13 legacy VERIFIED: actual callable path, expected reference,
   негативные сценарии и evidence_ref. До этого их labels не использовать как KPI.
   По результату либо приложить доказательства, либо явно переоткрыть row.
3. Перечислить реальные cases вместо `1/1` на stub; сохранить audit history,
   обнулить только неподтверждённые cases с объяснением.
4. Разделить `CONFIG-002` и `STORAGE-001`/дубли, broad CONVERTER-006 и API wildcards
   на независимые поведения. Parent строки не должны двойным счётом улучшать KPI.
5. Уточнить добавленный audit row ADAPTER-015 для **ezsp**: upstream
   `Adapter.create` имеет семь вариантов. До аудита global inventory содержала
   шесть, причём Ember ошибочно назван EZSP; исправить старое название при repair.
6. Аудировать все 16 доменов AGENTS: входы, handlers, tests, config properties,
   dependencies и docs. Опубликовать перечень просмотренных/непросмотренных
   surfaces, не «6/16» без списка и доказательств.
7. Каталог: отдельные machine-readable ID `[package SHA,module,index]`,
   definition count, whiteLabel count, converter behaviors, prepared exposes,
   exercised scenarios. Vendor files, aliases и models не равны устройствам.

**Gate:** schema validator проверяет полные source pins, уникальность ID, positive
weights, `0<=verified<=total`, source/test/evidence existence, approvals для
exclusions, запрет VERIFIED без evidence и hardware там, где требуется.

## EVID-03 — исправить генератор dashboard

Текущий `scripts/parity_report.py` только печатает арифметику; validation,
freshness, нулевые знаменатели и генерация Markdown не реализованы.

- stage1 сначала фильтровать по global eligibility, затем stage=1;
  `OUT_OF_SCOPE_APPROVED` нельзя случайно оставить в stage1 denominator.
- `N/A` для пустых subsets; раздельно case coverage и целиком VERIFIED rows;
  malformed или unsupported claims → nonzero без публикации ложного процента.
- Вывод: baseline/latest/per-repo freshness, audited domains, pending deltas,
  counts/weights/scenarios обоих KPI, R1 acceptance отдельно, exclusions,
  last tests, catalog и hardware matrix.
- Regression fixtures валидатора: duplicate ID, shifted column, отсутствующий
  evidence, mixed SHA, zero denominator, approved exclusion, stale fetch,
  mock-only hardware VERIFIED, case count больше total.
- CI проверяет generated-file diff; дата и результаты берутся из evidence,
  не из комментариев и не из даты написания отчёта.

## EVID-04 — reference/differential harness

1. Pinned Node app + зависимости, fake adapter или replay transport, отдельный
   broker, фиксированные config/database и stimuli. Никогда не два coordinator
   процесса одновременно на пользовательском USB.
2. Сохранить raw Zigbee bytes, decoded msg, MQTT topics/payload/retain/QoS,
   command results, errors/log categories, file snapshots и causal event order.
3. Golden provenance: source SHA, Node/package versions, fixture command,
   expected JSON types, timestamps tolerance, capture mode, license.
4. Normalization только whitelist (например абсолютный временный path или
   действительно nondeterministic timestamp). Не сортировать события и не
   удалять endpoint/manufacturer/поля ошибок ради совпадения.
5. Один stimulus прогоняется через Node и Rust; сравнение артефактов обязательно
   для изменяемых contracts. Stateful sequences важнее отдельных assert JSON.

## EVID-05 — CI gates

| Gate | Когда | Что блокирует |
|---|---|---|
| fmt/test/clippy | PR/main | regression/build quality; warnings устранить отдельным PR |
| Linux release + ARM GNU + OpenWrt musl | PR/release | build и target/native-dependency drift |
| inventory/evidence/report | каждый PR | неподдерживаемые parity claims |
| baseline Node + differential | контрактные изменения | несовпадения observable behavior |
| converter staged-runtime smoke | каждое package publication | broken layout/runtime/metadata |
| frontend browser E2E | API/frontend changes | ложная совместимость UI |
| periodic upstream refresh | ежедневно | очередь drift; не маскировать как Rust failure |
| HIL/soak | release candidate | R1 профиль Dongle-P; полный S1 отдельно |

Сейчас в `.github/workflows` только converter publication. Добавлять gates
поэтапно, не выдавать существующий publish job за Rust CI. Artifacts с логами
сохранять и при failure. Secrets/token не помещать в reference fixtures.
