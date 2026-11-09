// Original: JavaScript `Date` helpers used by
//           .upstream/zigbee-herdsman/src/utils/logger.ts (`new Date().toISOString()`)
//           .upstream/zigbee-herdsman/src/utils/backup.ts (`new Date().toISOString()`)
//           .upstream/zigbee2mqtt/lib/util/logger.ts (`Date`, `%TIMESTAMP%`)
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::time::{SystemTime, UNIX_EPOCH};

/// Upstream `Date.now()` in milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Upstream `new Date().toISOString()`.
pub fn now_iso8601() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    iso8601_from_unix(elapsed.as_secs() as i64, elapsed.subsec_millis())
}

/// Upstream `logger.ts` `%TIMESTAMP%` for the current time.
pub fn now_file_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    file_timestamp(seconds)
}

/// Format Unix seconds + milliseconds as `YYYY-MM-DDTHH:mm:ss.SSSZ`.
pub fn iso8601_from_unix(seconds: i64, millis: u32) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Upstream `logger.ts` `%TIMESTAMP%`: `YYYY-MM-DD.HH-mm-ss`.
pub fn file_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}.{hour:02}-{minute:02}-{second:02}")
}

/// Days since 1970-01-01 to a proleptic Gregorian date
/// (Howard Hinnant's `civil_from_days`).
pub fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}
