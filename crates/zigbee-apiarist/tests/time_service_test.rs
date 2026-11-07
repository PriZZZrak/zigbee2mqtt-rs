// Original: .upstream/zigbee-herdsman/test/timeService.test.ts

use zigbee_apiarist::utils::time_service::TimeService;

struct DateCase {
    time_zone: &'static str,
    local_time: &'static str,
    expected_time: u64,
    expected_time_zone: i32,
    expected_dst_start: u32,
    expected_dst_end: u32,
    expected_dst_shift: i32,
    expected_standard_time: u64,
    expected_local_time: u64,
}

fn date_cases() -> Vec<DateCase> {
    vec![
        DateCase {
            time_zone: "Atlantic/Reykjavik",
            local_time: "Wed Oct 01 2025 17:06:08 GMT+0000 (Greenwich Mean Time)",
            expected_time: 812653568,
            expected_time_zone: 0,
            expected_dst_start: 0xffffffff,
            expected_dst_end: 0xffffffff,
            expected_dst_shift: 0,
            expected_standard_time: 812653568,
            expected_local_time: 812653568,
        },
        DateCase {
            time_zone: "Europe/Berlin",
            local_time: "Wed Oct 01 2025 18:49:34 GMT+0200 (Central European Summer Time)",
            expected_time: 812652574,
            expected_time_zone: 3600,
            expected_dst_start: 796611600,
            expected_dst_end: 814755599,
            expected_dst_shift: 3600,
            expected_standard_time: 812656174,
            expected_local_time: 812659774,
        },
        DateCase {
            time_zone: "Europe/Berlin",
            local_time: "Mon Mar 02 2026 19:04:12 GMT+0100 (Central European Standard Time)",
            expected_time: 825789852,
            expected_time_zone: 3600,
            expected_dst_start: 828061200,
            expected_dst_end: 846205199,
            expected_dst_shift: 3600,
            expected_standard_time: 825793452,
            expected_local_time: 825793452,
        },
        DateCase {
            time_zone: "Australia/Sydney",
            local_time: "Thu Oct 02 2025 03:03:25 GMT+1000 (Australian Eastern Standard Time)",
            expected_time: 812653405,
            expected_time_zone: 36000,
            expected_dst_start: 812908800,
            expected_dst_end: 828633599,
            expected_dst_shift: 3600,
            expected_standard_time: 812689405,
            expected_local_time: 812689405,
        },
        DateCase {
            time_zone: "Australia/Sydney",
            local_time: "Mon Nov 03 2025 03:07:49 GMT+1100 (Australian Eastern Daylight Time)",
            expected_time: 815414869,
            expected_time_zone: 36000,
            expected_dst_start: 812908800,
            expected_dst_end: 828633599,
            expected_dst_shift: 3600,
            expected_standard_time: 815450869,
            expected_local_time: 815454469,
        },
        DateCase {
            time_zone: "Australia/Sydney",
            local_time: "Mon Jan 19 2026 03:08:11 GMT+1100 (Australian Eastern Daylight Time)",
            expected_time: 822067691,
            expected_time_zone: 36000,
            expected_dst_start: 812908800,
            expected_dst_end: 828633599,
            expected_dst_shift: 3600,
            expected_standard_time: 822103691,
            expected_local_time: 822107291,
        },
    ]
}

#[test]
#[ignore = "not implemented yet"]
fn should_work_correctly_for_specific_dates() {
    for case in date_cases() {
        // upstream `vi.stubEnv("TZ", timeZone)` + `vi.setSystemTime(new Date(localTime))`
        let mut time_service = TimeService::new(case.time_zone, case.expected_time);

        let attributes = time_service.get_time_cluster_attributes();

        assert_eq!(attributes.time, case.expected_time);
        assert_eq!(attributes.time_status, 0b1101);
        assert_eq!(attributes.time_zone, case.expected_time_zone);
        assert_eq!(attributes.dst_start, case.expected_dst_start);
        assert_eq!(attributes.dst_end, case.expected_dst_end);
        assert_eq!(attributes.dst_shift, case.expected_dst_shift);
        assert_eq!(attributes.standard_time, case.expected_standard_time);
        assert_eq!(attributes.local_time, case.expected_local_time);
        assert_eq!(attributes.last_set_time, case.expected_time);
        assert_eq!(
            attributes.valid_until_time,
            case.expected_time + 24 * 60 * 60
        );
    }
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_daylight_saving_time_correctly() {
    let cases = [
        ("Europe/Berlin", 0, 3600),
        ("Europe/Berlin", 3600, 0),
        ("Australia/Sydney", 0, 3600),
        ("Australia/Sydney", 3600, 0),
    ];

    for (time_zone, expected_shift_before, expected_shift_after) in cases {
        let mut time_service = TimeService::new(time_zone, 0);

        let first_run = time_service.get_time_cluster_attributes();
        // 1 second later
        time_service.advance(1);
        let second_run = time_service.get_time_cluster_attributes();

        assert_eq!(
            first_run.local_time as i64 - first_run.standard_time as i64,
            expected_shift_before
        );
        assert_eq!(
            second_run.local_time as i64 - second_run.standard_time as i64,
            expected_shift_after
        );
    }
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_use_cached_data_for_dynamic_attributes() {
    let mut time_service = TimeService::new("Europe/Berlin", 0);
    let first_run = time_service.get_time_cluster_attributes();

    // 1 hour later
    let delta = 60 * 60;
    time_service.advance(delta);
    let second_run = time_service.get_time_cluster_attributes();

    assert_eq!(second_run.time, first_run.time + delta);
    assert_eq!(second_run.standard_time, first_run.standard_time + delta);
    assert_eq!(second_run.local_time, first_run.local_time + delta);
    assert_eq!(second_run.last_set_time, first_run.last_set_time + delta);
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_cached_information_within_24_hours() {
    let mut time_service = TimeService::new("Europe/Berlin", 0);
    let first_run = time_service.get_time_cluster_attributes();

    // 23:59:59 later
    time_service.advance(23 * 60 * 60 + 59 * 60 + 59);
    let second_run = time_service.get_time_cluster_attributes();

    assert_eq!(second_run.valid_until_time, first_run.valid_until_time);
}

#[test]
#[ignore = "not implemented yet"]
fn should_recalculate_the_cache_after_24_hours() {
    let mut time_service = TimeService::new("Europe/Berlin", 0);
    let first_run = time_service.get_time_cluster_attributes();

    // 24 hours later
    time_service.advance(24 * 60 * 60);
    let second_run = time_service.get_time_cluster_attributes();

    assert_ne!(second_run.valid_until_time, first_run.valid_until_time);
}
