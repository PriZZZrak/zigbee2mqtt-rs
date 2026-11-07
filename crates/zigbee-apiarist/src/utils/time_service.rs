// Original: .upstream/zigbee-herdsman/src/utils/timeService.ts
// TODO: port timezone/DST calculation and caching.

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

/// Model of the module-level cache in `src/utils/timeService.ts`.
#[derive(Debug, Clone)]
pub struct TimeService {
    /// Upstream `TZ` environment.
    pub timezone: String,
    /// Current system time in seconds since epoch.
    pub now: u64,
    /// Cached cluster attributes (upstream module-level `cachedTimeClusterAttributes`).
    pub cache: Option<TimeClusterAttributes>,
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
        todo!()
    }
}

pub fn clear_cached_time_data() {
    todo!()
}

/// Upstream `timestampToZigbeeUtcTime()`.
pub fn timestamp_to_zigbee_utc_time(_timestamp: u64) -> u64 {
    todo!()
}
