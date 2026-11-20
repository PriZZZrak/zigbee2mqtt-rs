// Z2M external converter — Lytko L101Ze.v2 и варианты (-S, -TT, -TS, -SS), firmware 1.5.x … 2.0.1
// 2.0.1: отрицательная коррекция снова доходит до прибора; кнопка «Вернуться в WiFi» (0x0204/0x4901).
// Источник истины по атрибутам: components/LZB-thermostat/src/{LZB_thermostat,LZB_switch}.c
// Manuf code 0x7777 (30583). Vendor-атрибуты — на hvacThermostat и genOnOffSwitchCfg.
//
// Установка: Z2M → Settings → External converters → добавить этот файл.
//
// Пять моделей одной прошивочной линейки (роль канала зашита сборкой):
//   L101Ze.v2      1 канал, термостат (исторический: свойства БЕЗ суффиксов)
//   L101Ze.v2-S    1 канал, выключатель
//   L101Ze.v2-TT   2 канала, термостат + термостат
//   L101Ze.v2-TS   2 канала, термостат + выключатель
//   L101Ze.v2-SS   2 канала, выключатель + выключатель
// Кнопка на EP3 (Multistate Input, события single/double/hold) — только у
// ОДНОКАНАЛЬНЫХ: один прибор — две сущности, у двухканального вторую занимает
// второй канал.
// Каналы: EP1=l1, EP2=l2. Выключательный канал: On/Off + тип контакта +
// отвязка от реле (decoupled, vendor 0xFF10) + события провода (коды 1..5:
// нажатия и замкнулся/разомкнулся) + ПОЛОЖЕНИЕ провода (genBinaryInput 0x000F,
// свойство contact_*) — им и работает геркон.
//
// ВАЖНО: репортинг на genMultistateInput НЕ настраивать (риск R5 плана):
// периодика воспроизводила бы последнее нажатие как фантомное. Прошивка шлёт
// события сама, явными кадрами.

const fz = require('zigbee-herdsman-converters/converters/fromZigbee');
const tz = require('zigbee-herdsman-converters/converters/toZigbee');
const exposes = require('zigbee-herdsman-converters/lib/exposes');
const reporting = require('zigbee-herdsman-converters/lib/reporting');

const e = exposes.presets;
const ea = exposes.access;

const MANUF = {manufacturerCode: 0x7777};

// Vendor-атрибуты на hvacThermostat (0x0201)
const A = {
    SENSOR_TYPE: 0xFF00,  // enum 0..9, порядок L_adc
    HYSTERESIS:  0xFF01,  // U8, десятые доли °C (5..30 = 0.5..3.0°)
    CORRECTION:  0xFF02,  // S8, десятые доли °C (-50..50 = -5..+5°)
    FAULT_CODE:  0xFF05,  // enum 0..7, приоритетная авария (ns_fault_code)
};

// Electrical Measurement (0x0B04) — ток/мощность/маска аварий
const EM = {
    RMS_CURRENT:    0x0508,  // мА
    APPARENT_POWER: 0x050F,  // ВА
    ALARMS_MASK:    0x0800,  // 16-битная маска ns_fault
};

// Порядок ровно как s_sensor_names[] в main/app_zigbee_main.c
const SENSOR_TYPES = [
    '3.3 кОм', '5 кОм', '6.8 кОм', '10 кОм', '12 кОм',
    '14.8 кОм', '15 кОм', '20 кОм', '33 кОм', '47 кОм',
];

// Коды NS_FAULT_CODE_* из main/NS/ns_fault.h
const FAULT_CODES = [
    'Нет аварий',
    'Нагрев при выключенном реле',
    'Перегруз',
    'Превышение тока',
    'Обрыв нагревателя',
    'Ошибка датчика температуры',
    'Ток невалиден',
    'Деградация нагревателя',
];

// Биты маски NS_FAULT_* — раскладываем в отдельные булевы для автоматизаций
const FAULT_BITS = {
    fault_heating_while_off: 0x01,
    fault_heater_open:       0x02,
    fault_overcurrent:       0x04,
    fault_element_degraded:  0x08,
    fault_sensor:            0x10,
    fault_current_invalid:   0x20,
    fault_overload_trip:     0x40,
};

const onOff = (v) => (v ? 'ON' : 'OFF');

// Наши значения SwitchType (ns_contact_type_t), НЕ ZCL-ные
const CONTACT_TYPES = ['Кнопка', 'Выключатель', 'Переключатель'];

// Коды PresentValue → имя действия (события провода и кнопки)
// Коды 4/5 — фронты ПРОВОДА, а не состояние реле. Имена press/release такие же,
// как у wifi-версии в Home Assistant: одно и то же событие зовётся одинаково.
const ACTION_CODES = {1: 'single', 2: 'double', 3: 'hold', 4: 'press', 5: 'release'};

// Имя эндпоинта по карте модели; null у моделей без карты (исторический t)
const epName = (model, msg) => {
    if (typeof model.endpoint !== 'function') return null;
    const map = model.endpoint();
    for (const [name, id] of Object.entries(map)) if (id === msg.endpoint.ID) return name;
    return null;
};

// Суффикс свойств по эндпоинту — только у многоэндпоинтных моделей
const suffixed = (model, msg, payload) => {
    const n = epName(model, msg);
    if (!n) return payload;
    const out = {};
    for (const [k, v] of Object.entries(payload)) out[`${k}_${n}`] = v;
    return out;
};

const fzLocal = {
    lytko_thermostat_vendor: {
        cluster: 'hvacThermostat',
        type: ['attributeReport', 'readResponse'],
        convert: (model, msg) => {
            const d = msg.data;
            const out = {};
            const get = (id) => (d[id] !== undefined ? d[id] : d[String(id)]);

            let v;
            if ((v = get(A.SENSOR_TYPE)) !== undefined) out.sensor_type = SENSOR_TYPES[v] ?? String(v);
            if ((v = get(A.HYSTERESIS)) !== undefined) out.hysteresis = v / 10;
            // S8 приезжает беззнаковым — доворачиваем сами
            if ((v = get(A.CORRECTION)) !== undefined) out.correction = (v > 127 ? v - 256 : v) / 10;
            if ((v = get(A.FAULT_CODE)) !== undefined) out.fault_reason = FAULT_CODES[v] ?? String(v);
            return suffixed(model, msg, out);
        },
    },

    // События провода (EP каналов) и кнопки (EP3): каждый report = действие.
    // Фильтра «по изменению» нет НАМЕРЕННО: два одиночных подряд — два действия.
    lytko_action: {
        cluster: 'genMultistateInput',
        type: ['attributeReport'],
        convert: (model, msg) => {
            const code = msg.data.presentValue;
            const name = ACTION_CODES[code];
            if (!name) return;
            const n = epName(model, msg);
            return {action: n ? `${name}_${n}` : name};
        },
    },

    // Положение провода (genBinaryInput 0x000F) — «геркон». Отдельно от
    // действий: это состояние, его можно перечитать и увидеть при старте.
    lytko_contact: {
        cluster: 'genBinaryInput',
        type: ['attributeReport', 'readResponse'],
        convert: (model, msg) => {
            if (msg.data.presentValue === undefined) return;
            const v = msg.data.presentValue ? 'closed' : 'open';
            const n = epName(model, msg);
            return {[n ? `contact_${n}` : 'contact']: v};
        },
    },

    // Тип контакта и отвязка (genOnOffSwitchCfg 0x0007)
    lytko_switch_cfg: {
        cluster: 'genOnOffSwitchCfg',
        type: ['attributeReport', 'readResponse'],
        convert: (model, msg) => {
            const d = msg.data;
            const out = {};
            if (d.switchType !== undefined) out.switch_type = CONTACT_TYPES[d.switchType] ?? String(d.switchType);
            const dec = d[0xFF10] !== undefined ? d[0xFF10] : d[String(0xFF10)];
            if (dec !== undefined) out.decoupled = onOff(dec);
            return suffixed(model, msg, out);
        },
    },

    lytko_metering: {
        cluster: 'haElectricalMeasurement',
        type: ['attributeReport', 'readResponse'],
        convert: (model, msg) => {
            const d = msg.data;
            const out = {};
            const get = (id) => (d[id] !== undefined ? d[id] : d[String(id)]);

            let v;
            if ((v = get(EM.RMS_CURRENT)) !== undefined) out.current = v / 1000;   // делитель 1000
            if ((v = get(EM.APPARENT_POWER)) !== undefined) out.power = v;         // делитель 1
            if ((v = get(EM.ALARMS_MASK)) !== undefined) {
                out.fault_mask = v;
                for (const [prop, bit] of Object.entries(FAULT_BITS)) out[prop] = onOff(v & bit);
            }
            return suffixed(model, msg, out);
        },
    },

    lytko_energy: {
        cluster: 'seMetering',
        type: ['attributeReport', 'readResponse'],
        convert: (model, msg) => {
            const v = msg.data.currentSummDelivered;
            if (v === undefined) return {};
            // U48 приезжает парой [старшие, младшие]; прошивка копит в Вт·ч
            const wh = Array.isArray(v) ? (v[0] * 0x100000000 + v[1]) : v;
            return suffixed(model, msg, {energy: wh / 1000});
        },
    },
};

const tzLocal = {
    lytko_sensor_type: {
        key: ['sensor_type'],
        convertSet: async (entity, key, value) => {
            const idx = SENSOR_TYPES.indexOf(String(value));
            if (idx < 0) throw new Error(`sensor_type: ${value} не из ${SENSOR_TYPES}`);
            await entity.write('hvacThermostat', {[A.SENSOR_TYPE]: {value: idx, type: 0x30}}, MANUF);
            return {state: {sensor_type: SENSOR_TYPES[idx]}};
        },
        convertGet: async (entity) => {
            await entity.read('hvacThermostat', [A.SENSOR_TYPE], MANUF);
        },
    },

    lytko_hysteresis: {
        key: ['hysteresis'],
        convertSet: async (entity, key, value) => {
            // Прошивка квантует к шагу 0.5° и зажимает в 0.5..3.0
            let raw = Math.round(Number(value) * 10 / 5) * 5;
            if (raw < 5) raw = 5;
            if (raw > 30) raw = 30;
            await entity.write('hvacThermostat', {[A.HYSTERESIS]: {value: raw, type: 0x20}}, MANUF);
            return {state: {hysteresis: raw / 10}};
        },
        convertGet: async (entity) => {
            await entity.read('hvacThermostat', [A.HYSTERESIS], MANUF);
        },
    },

    lytko_correction: {
        key: ['correction'],
        convertSet: async (entity, key, value) => {
            // Прошивка округляет к целым градусам и зажимает в -5..+5
            let deg = Math.round(Number(value));
            if (deg > 5) deg = 5;
            if (deg < -5) deg = -5;
            const raw = deg * 10;
            await entity.write('hvacThermostat',
                // Тип 0x28 — знаковый байт: отрицательное шлём как есть. Раньше слали raw+256
                // (−2° → 236), z2m отвергал запись (RangeError), до прибора не доходило ничего.
                {[A.CORRECTION]: {value: raw, type: 0x28}}, MANUF);
            return {state: {correction: deg}};
        },
        convertGet: async (entity) => {
            await entity.read('hvacThermostat', [A.CORRECTION], MANUF);
        },
    },
};

tzLocal.lytko_switch_type = {
    key: ['switch_type'],
    convertSet: async (entity, key, value) => {
        const idx = CONTACT_TYPES.indexOf(String(value));
        if (idx < 0) throw new Error(`switch_type: ${value} не из ${CONTACT_TYPES}`);
        await entity.write('genOnOffSwitchCfg', {switchType: idx});
        return {state: {switch_type: CONTACT_TYPES[idx]}};
    },
    convertGet: async (entity) => {
        await entity.read('genOnOffSwitchCfg', ['switchType']);
    },
};

// «Вернуться в WiFi» — как у 102 (L102Ze-*.js lytko_recovery): разовая запись 1 в vendor 0x4901
// кластера 0x0204. Прошивка слушает только EP1, сама сбрасывает атрибут и перезагружается в
// соседний ota-слот с WiFi-прошивкой. Тумблер оптимистично «отскакивает» в OFF.
tzLocal.lytko_recovery = {
    key: ['recovery'],
    convertSet: async (entity, key, value, meta) => {
        const v = (value === 'ON' || value === true || value === 1) ? 1 : 0;
        const ep = (meta && meta.device && meta.device.getEndpoint(1)) || entity;
        if (v) await ep.write('hvacUserInterfaceCfg', {0x4901: {value: 1, type: 0x30}}, MANUF);
        return {state: {recovery: 'OFF'}};
    },
};

const recoveryExpose = () => e.binary('recovery', ea.STATE_SET, 'ON', 'OFF').withLabel('Вернуться в WiFi')
    .withDescription('Возврат в WiFi-прошивку: прибор перезагрузится в соседний раздел. '
        + 'Разовый триггер — тумблер сам отскакивает.');

tzLocal.lytko_decoupled = {
    key: ['decoupled'],
    convertSet: async (entity, key, value) => {
        const on = String(value).toUpperCase() === 'ON' || value === true;
        await entity.write('genOnOffSwitchCfg', {[0xFF10]: {value: on ? 1 : 0, type: 0x10}}, MANUF);
        return {state: {decoupled: on ? 'ON' : 'OFF'}};
    },
    convertGet: async (entity) => {
        await entity.read('genOnOffSwitchCfg', [0xFF10], MANUF);
    },
};

/* ── Конструкторы кусков ── */

// Термостатные exposes; n — имя эндпоинта (null у исторического t)
const thermoExposes = (n) => {
    const w = (x) => (n ? x.withEndpoint(n) : x);
    return [
        w(e.climate()
            .withSetpoint('occupied_heating_setpoint', 5, 40, 0.5)
            .withLocalTemperature()
            .withSystemMode(['off', 'heat'])
            .withRunningState(['idle', 'heat'])),
        w(e.enum('sensor_type', ea.ALL, SENSOR_TYPES).withLabel('Тип датчика')
            .withDescription('Номинал NTC-датчика пола. Неверный тип — неверная температура.')),
        w(e.numeric('hysteresis', ea.ALL).withValueMin(0.5).withValueMax(3).withValueStep(0.5)
            .withUnit('°C').withLabel('Гистерезис')
            .withDescription('Насколько температура должна отклониться, прежде чем сработает реле.')),
        w(e.numeric('correction', ea.ALL).withValueMin(-5).withValueMax(5).withValueStep(1)
            .withUnit('°C').withLabel('Коррекция')
            .withDescription('Смещение показаний датчика.')),
    ];
};

// Выключательный канал: нагрузка + тип контакта + отвязка
const switchExposes = (n) => [
    e.switch().withEndpoint(n),
    e.enum('switch_type', ea.ALL, CONTACT_TYPES).withEndpoint(n).withLabel('Тип контакта')
        .withDescription('Кнопка: клик переключает. Выключатель: положение = состояние. '
            + 'Переключатель: каждое перекидывание меняет состояние (проходная схема).'),
    e.binary('decoupled', ea.ALL, 'ON', 'OFF').withEndpoint(n).withLabel('Отвязать от реле')
        .withDescription('Сигнал с провода идёт в сценарии, реле не реагирует.'),
];

const meterExposes = (n) => {
    const w = (x) => (n ? x.withEndpoint(n) : x);
    return [w(e.current()), w(e.power()), w(e.energy())];
};

const faultExposes = (n) => {
    const w = (x) => (n ? x.withEndpoint(n) : x);
    return [
        w(e.enum('fault_reason', ea.STATE, FAULT_CODES).withLabel('Причина аварии')),
        w(e.numeric('fault_mask', ea.STATE).withLabel('Маска аварий')),
        w(e.binary('fault_heating_while_off', ea.STATE, 'ON', 'OFF').withLabel('Нагрев при выкл. реле')),
        w(e.binary('fault_heater_open', ea.STATE, 'ON', 'OFF').withLabel('Обрыв нагревателя')),
        w(e.binary('fault_overcurrent', ea.STATE, 'ON', 'OFF').withLabel('Превышение тока')),
        w(e.binary('fault_element_degraded', ea.STATE, 'ON', 'OFF').withLabel('Деградация нагревателя')),
        w(e.binary('fault_sensor', ea.STATE, 'ON', 'OFF').withLabel('Ошибка датчика температуры')),
        w(e.binary('fault_current_invalid', ea.STATE, 'ON', 'OFF').withLabel('Ток невалиден')),
        w(e.binary('fault_overload_trip', ea.STATE, 'ON', 'OFF').withLabel('Сработала защита по перегрузу')),
    ];
};

const wireActionNames = (n) => ['single', 'double', 'hold', 'press', 'release'].map((a) => `${a}_${n}`);

// Положение провода: у канала-выключателя и у кнопки. Термостатному каналу
// нечего показывать — там на проводе датчик пола.
const contactExpose = (n) => e.binary(n ? `contact_${n}` : 'contact', ea.STATE, 'closed', 'open')
    .withLabel(n ? `Контакт ${n === 'button' ? 'кнопки' : n.toUpperCase()}` : 'Контакт')
    .withDescription('Положение провода прямо сейчас: замкнут или разомкнут. '
        + 'Для геркона на двери: closed — закрыта.');
// У кнопки те же пять кодов, что у провода канала: жесты и фронты.
const btnActionNames = (multi) => (multi
    ? ['single_button', 'double_button', 'hold_button', 'press_button', 'release_button']
    : ['single', 'double', 'hold', 'press', 'release']);

const FZ_ALL = [fz.thermostat, fz.on_off, fzLocal.lytko_thermostat_vendor,
    fzLocal.lytko_switch_cfg, fzLocal.lytko_metering, fzLocal.lytko_energy,
    fzLocal.lytko_action, fzLocal.lytko_contact];
const TZ_ALL = [tz.thermostat_occupied_heating_setpoint, tz.thermostat_system_mode,
    tz.thermostat_local_temperature, tz.thermostat_running_state, tz.on_off,
    tzLocal.lytko_recovery,
    tzLocal.lytko_sensor_type, tzLocal.lytko_hysteresis, tzLocal.lytko_correction,
    tzLocal.lytko_switch_type, tzLocal.lytko_decoupled];

// roles: массив 'T'|'S' по каналам
const makeDefinition = (model, description, roles) => {
    const multi = true;
    /* Кнопка (EP3) есть только у одноканальных исполнений: один прибор — две
       сущности, и в двухканальном вторую занимает второй канал. */
    const hasButton = roles.length === 1;
    const map = hasButton ? {button: 3} : {};
    roles.forEach((r, i) => { map[`l${i + 1}`] = i + 1; });

    const ex = [];
    const actions = hasButton ? [...btnActionNames(multi)] : [];
    if (hasButton) ex.push(contactExpose('button'));
    roles.forEach((r, i) => {
        const n = `l${i + 1}`;
        if (r === 'T') ex.push(...thermoExposes(n));
        else {
            ex.push(...switchExposes(n), contactExpose(n));   /* положение провода */
            actions.push(...wireActionNames(n));
        }
        ex.push(...meterExposes(n), ...faultExposes(n));
    });
    /* У двухканального термостата действий нет вовсе (оба провода под датчиками) —
       пустой список сущности в хабе не нужен. */
    if (actions.length) ex.push(e.action(actions));
    ex.push(recoveryExpose());                   /* один на прибор — атрибут на EP1 */

    return {
        zigbeeModel: [model],
        model,
        vendor: 'Lytko',
        description,
        fromZigbee: FZ_ALL,
        toZigbee: TZ_ALL,
        exposes: ex,
        endpoint: () => map,
        meta: {multiEndpoint: true},
        configure: async (device, coordinatorEndpoint) => {
            for (let i = 0; i < roles.length; i++) {
                const ep = device.getEndpoint(i + 1);
                if (!ep) continue;
                const clusters = roles[i] === 'T'
                    ? ['hvacThermostat', 'haElectricalMeasurement', 'seMetering']
                    : ['genOnOff', 'genBinaryInput', 'haElectricalMeasurement', 'seMetering'];
                // genMultistateInput НЕ биндим и репортинг НЕ настраиваем (R5)
                try { await reporting.bind(ep, coordinatorEndpoint, clusters); } catch (err) {/* прошивка шлёт сама */}
                if (roles[i] === 'T') {
                    await ep.read('hvacThermostat',
                        ['localTemp', 'occupiedHeatingSetpoint', 'systemMode', 'runningState']).catch(() => {});
                    await ep.read('hvacThermostat',
                        [A.SENSOR_TYPE, A.HYSTERESIS, A.CORRECTION, A.FAULT_CODE], MANUF).catch(() => {});
                } else {
                    await ep.read('genOnOff', ['onOff', 'startUpOnOff']).catch(() => {});
                    await ep.read('genOnOffSwitchCfg', ['switchType']).catch(() => {});
                    await ep.read('genOnOffSwitchCfg', [0xFF10], MANUF).catch(() => {});
                    // Положение провода: спрашиваем сразу, чтобы геркон не ждал
                    // первого движения двери.
                    await ep.read('genBinaryInput', ['presentValue']).catch(() => {});
                }
                await ep.read('haElectricalMeasurement',
                    [EM.RMS_CURRENT, EM.APPARENT_POWER, EM.ALARMS_MASK]).catch(() => {});
                await ep.read('seMetering', ['currentSummDelivered']).catch(() => {});
            }
            if (hasButton) {
                const epb = device.getEndpoint(3);
                if (epb) {
                    try { await reporting.bind(epb, coordinatorEndpoint, ['genBinaryInput']); }
                    catch (err) {/* прошивка шлёт сама */}
                    await epb.read('genBinaryInput', ['presentValue']).catch(() => {});
                }
            }
        },
    };
};

/* Исторический одноканальный термостат: свойства БЕЗ суффиксов, карты
   эндпоинтов НЕТ — иначе у стоящих в поле приборов переименовались бы все
   сущности Home Assistant. Кнопка добавляет только action. */
const definition = {
    zigbeeModel: ['L101Ze.v2'],
    model: 'L101Ze.v2',
    vendor: 'Lytko',
    description: 'Термостат тёплого пола Lytko 101 NS, Zigbee',
    fromZigbee: [
        fz.thermostat,
        fzLocal.lytko_thermostat_vendor,
        fzLocal.lytko_metering,
        fzLocal.lytko_energy,
        fzLocal.lytko_action,
        fzLocal.lytko_contact,
    ],
    toZigbee: [
        tz.thermostat_occupied_heating_setpoint,
        tz.thermostat_system_mode,
        tz.thermostat_local_temperature,
        tz.thermostat_running_state,
        tzLocal.lytko_sensor_type,
        tzLocal.lytko_hysteresis,
        tzLocal.lytko_correction,
        tzLocal.lytko_recovery,
    ],
    exposes: [
        e.climate()
            .withSetpoint('occupied_heating_setpoint', 5, 40, 0.5)
            .withLocalTemperature()
            .withSystemMode(['off', 'heat'])
            .withRunningState(['idle', 'heat']),

        e.enum('sensor_type', ea.ALL, SENSOR_TYPES).withLabel('Тип датчика')
            .withDescription('Номинал NTC-датчика пола. Неверный тип — неверная температура.'),
        e.numeric('hysteresis', ea.ALL).withValueMin(0.5).withValueMax(3).withValueStep(0.5)
            .withUnit('°C').withLabel('Гистерезис')
            .withDescription('Насколько температура должна отклониться, прежде чем сработает реле. '
                + 'Меньше — точнее, но чаще щёлкает реле.'),
        e.numeric('correction', ea.ALL).withValueMin(-5).withValueMax(5).withValueStep(1)
            .withUnit('°C').withLabel('Коррекция')
            .withDescription('Смещение показаний датчика. Если прибор показывает на 2° выше '
                + 'реальной температуры пола — поставьте -2.'),

        e.current(), e.power(), e.energy(),

        e.enum('fault_reason', ea.STATE, FAULT_CODES).withLabel('Причина аварии')
            .withDescription('Самая важная из активных аварий. Все сразу — в fault_mask.'),
        e.numeric('fault_mask', ea.STATE).withLabel('Маска аварий')
            .withDescription('Битовая маска: 0x01 нагрев при выкл. реле, 0x02 обрыв, '
                + '0x04 превышение тока, 0x08 деградация, 0x10 датчик, 0x20 ток невалиден, '
                + '0x40 перегруз.'),
        e.binary('fault_heating_while_off', ea.STATE, 'ON', 'OFF').withLabel('Нагрев при выкл. реле'),
        e.binary('fault_heater_open', ea.STATE, 'ON', 'OFF').withLabel('Обрыв нагревателя'),
        e.binary('fault_overcurrent', ea.STATE, 'ON', 'OFF').withLabel('Превышение тока'),
        e.binary('fault_element_degraded', ea.STATE, 'ON', 'OFF').withLabel('Деградация нагревателя'),
        e.binary('fault_sensor', ea.STATE, 'ON', 'OFF').withLabel('Ошибка датчика температуры'),
        e.binary('fault_current_invalid', ea.STATE, 'ON', 'OFF').withLabel('Ток невалиден'),
        e.binary('fault_overload_trip', ea.STATE, 'ON', 'OFF').withLabel('Сработала защита по перегрузу'),

        e.action(btnActionNames(false)).withDescription('Виртуальный вход — отдельная кнопка на двух проводах (EP3)'),
        contactExpose(null),
        recoveryExpose(),
    ],
    configure: async (device, coordinatorEndpoint) => {
        const ep1 = device.getEndpoint(1);
        try {
            await reporting.bind(ep1, coordinatorEndpoint,
                ['hvacThermostat', 'haElectricalMeasurement', 'seMetering']);
        } catch (err) {/* не фатально: прошивка шлёт репорты сама */}

        await ep1.read('hvacThermostat',
            ['localTemp', 'occupiedHeatingSetpoint', 'systemMode', 'runningState']).catch(() => {});
        await ep1.read('hvacThermostat',
            [A.SENSOR_TYPE, A.HYSTERESIS, A.CORRECTION, A.FAULT_CODE], MANUF).catch(() => {});
        await ep1.read('haElectricalMeasurement',
            [EM.RMS_CURRENT, EM.APPARENT_POWER, EM.ALARMS_MASK]).catch(() => {});
        await ep1.read('seMetering', ['currentSummDelivered']).catch(() => {});

        // Кнопка на EP3: положение провода — привязка и первое чтение.
        const ep3 = device.getEndpoint(3);
        if (ep3) {
            try { await reporting.bind(ep3, coordinatorEndpoint, ['genBinaryInput']); }
            catch (err) {/* прошивка шлёт сама */}
            await ep3.read('genBinaryInput', ['presentValue']).catch(() => {});
        }
    },
    meta: {},
};

module.exports = [
    definition,
    makeDefinition('L101Ze.v2-S',  'Реле с проводным выключателем Lytko 101 NS, Zigbee', ['S']),
    makeDefinition('L101Ze.v2-TT', 'Термостат тёплого пола Lytko 101 NS, 2 канала, Zigbee', ['T', 'T']),
    makeDefinition('L101Ze.v2-TS', 'Термостат + реле Lytko 101 NS, 2 канала, Zigbee', ['T', 'S']),
    makeDefinition('L101Ze.v2-SS', 'Два реле с проводными выключателями Lytko 101 NS, Zigbee', ['S', 'S']),
];
