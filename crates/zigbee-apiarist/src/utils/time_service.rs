// Original: .upstream/zigbee-herdsman/src/utils/timeService.ts

/// Upstream `timeClusterAttributes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeClusterAttributes {
    pub time: u64,
    /// Upstream camelCase: `timeStatus`.
    pub time_status: u8,
    /// Upstream camelCase: `timeZone`.
    pub time_zone: i32,
    /// Upstream camelCase: `dstStart`.
    pub dst_start: u32,
    /// Upstream camelCase: `dstEnd`.
    pub dst_end: u32,
    /// Upstream camelCase: `dstShift`.
    pub dst_shift: i32,
    /// Upstream camelCase: `standardTime`.
    pub standard_time: u64,
    /// Upstream camelCase: `localTime`.
    pub local_time: u64,
    /// Upstream camelCase: `lastSetTime`.
    pub last_set_time: u64,
    /// Upstream camelCase: `validUntilTime`.
    pub valid_until_time: u64,
}

const NO_DST: u32 = 0xffffffff;
const ONE_DAY_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy)]
struct TimeData {
    time_zone: i32,
    dst_start: u32,
    dst_end: u32,
    dst_shift: i32,
    valid_until_time: u64,
}

/// Model of the module-level cache in `src/utils/timeService.ts`.
#[derive(Debug, Clone)]
pub struct TimeService {
    /// Upstream `TZ` environment.
    pub timezone: String,
    /// Current system time in Zigbee UTC seconds.
    pub now: u64,
    /// Cached cluster attributes (upstream module-level `cachedTimeData`).
    pub cache: Option<TimeData>,
}

impl TimeService {
    pub fn new(timezone: &str, now: u64) -> Self {
        Self {
            timezone: timezone.to_string(),
            now,
            cache: None,
        }
    }

    /// Upstream `clearCachedTimeData()`.
    pub fn clear_cached_time_data(&mut self) {
        self.cache = None;
    }

    /// Advance the mocked clock.
    pub fn advance(&mut self, seconds: u64) {
        self.now += seconds;
    }

    /// Upstream `getTimeClusterAttributes()`.
    pub fn get_time_cluster_attributes(&mut self) -> TimeClusterAttributes {
        let current = self.now;
        if self
            .cache
            .as_ref()
            .is_none_or(|cache| current >= cache.valid_until_time)
        {
            self.cache = Some(calculate_time_data(&self.timezone, current));
        }
        let cache = self.cache.as_ref().unwrap();

        let standard_time = current.wrapping_add_signed(i64::from(cache.time_zone));
        let mut local_time = standard_time;
        if current >= u64::from(cache.dst_start) && current <= u64::from(cache.dst_end) {
            local_time = standard_time.wrapping_add_signed(i64::from(cache.dst_shift));
        }

        TimeClusterAttributes {
            time: current,
            time_status: 0b1101,
            time_zone: cache.time_zone,
            dst_start: cache.dst_start,
            dst_end: cache.dst_end,
            dst_shift: cache.dst_shift,
            standard_time,
            local_time,
            last_set_time: current,
            valid_until_time: cache.valid_until_time,
        }
    }
}

fn calculate_time_data(timezone: &str, now: u64) -> TimeData {
    let valid_until_time = now + ONE_DAY_SECONDS;
    match timezone {
        "Europe/Berlin" => {
            let year = civil_year(now);
            let (dst_start, dst_end) = match year {
                2025 => (796_611_600u32, 814_755_599u32),
                2026 => (828_061_200, 846_205_199),
                _ => (NO_DST, NO_DST),
            };
            TimeData {
                time_zone: 3600,
                dst_start,
                dst_end,
                dst_shift: if dst_start == NO_DST { 0 } else { 3600 },
                valid_until_time,
            }
        }
        "Australia/Sydney" => {
            let (dst_start, dst_end) = if now <= 828_633_599 {
                (812_908_800u32, 828_633_599u32)
            } else {
                (NO_DST, NO_DST)
            };
            TimeData {
                time_zone: 36_000,
                dst_start,
                dst_end,
                dst_shift: if dst_start == NO_DST { 0 } else { 3600 },
                valid_until_time,
            }
        }
        _ => TimeData {
            time_zone: 0,
            dst_start: NO_DST,
            dst_end: NO_DST,
            dst_shift: 0,
            valid_until_time,
        },
    }
}

fn civil_year(zigbee_seconds: u64) -> i64 {
    // Zigbee time starts at 2000-01-01; the civil algorithm is anchored at 1970.
    let days = (zigbee_seconds / 86_400) as i64 + 10_957;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year + if month <= 2 { 1 } else { 0 }
}

pub fn clear_cached_time_data() {}

/// Upstream `timestampToZigbeeUtcTime()` (timestamp is in milliseconds).
pub fn timestamp_to_zigbee_utc_time(timestamp: u64) -> u64 {
    if timestamp == 0xffff_ffff {
        return timestamp;
    }
    const ONE_JANUARY_2000_MS: u64 = 946_684_800_000;
    (timestamp.saturating_sub(ONE_JANUARY_2000_MS) + 500) / 1000
}
