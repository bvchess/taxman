//! UTC timestamps for the records of a run: a result's `started` and `finished`, and the time of day that starts a
//! progress line.  Written by hand from [`SystemTime`], the date from the day count by the civil-from-days algorithm
//! (H. Hinnant, "chrono-Compatible Low-Level Date Algorithms"), so that the crate needs no date library.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds in a day.
const DAY: u64 = 86_400;

/// `time` in RFC 3339 and UTC, to the second: `2026-11-01T12:34:56Z`.  A time before 1970 is written as 1970.
#[must_use]
pub fn rfc3339(time: SystemTime) -> String {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let (year, month, day) = civil(seconds / DAY);
    let in_day = seconds % DAY;
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", in_day / 3600, in_day / 60 % 60, in_day % 60)
}

/// The time of day of `time` in UTC, to the second: `12:34:56Z`, the end of [`rfc3339`].
#[must_use]
pub fn time_of_day(time: SystemTime) -> String {
    // `rfc3339` is `YYYY-MM-DDT` (11 characters, as long as the year has four digits) and then the time of day.
    let stamp = rfc3339(time);
    stamp[stamp.find('T').map_or(0, |at| at + 1)..].to_owned()
}

/// The year, month (1 to 12) and day of the month (1 to 31) of the day `days` days after 1970-01-01.
fn civil(days: u64) -> (u64, u64, u64) {
    // Days from 0000-03-01, so that the leap day is the last of the year; eras are 400 years of 146,097 days.
    let z = days + 719_468;
    let (era, day_of_era) = (z / 146_097, z % 146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn a_time_is_written_in_rfc_3339_and_utc() {
        for (seconds, text) in [
            (0, "1970-01-01T00:00:00Z"),
            (1_000_000_000, "2001-09-09T01:46:40Z"),
            // The end of 1999, the leap day of 2000 (a year divisible by 400), and 2024's.
            (946_684_799, "1999-12-31T23:59:59Z"),
            (951_868_799, "2000-02-29T23:59:59Z"),
            (1_709_208_000, "2024-02-29T12:00:00Z"),
            // 2100 is no leap year, so it has no 29 February: the day after 28 February is 1 March.
            (4_107_542_400, "2100-03-01T00:00:00Z"),
            (1_793_536_496, "2026-11-01T12:34:56Z"),
        ] {
            assert_eq!(rfc3339(at(seconds)), text);
        }
    }

    #[test]
    fn a_time_before_1970_is_the_epoch_and_a_fraction_of_a_second_is_dropped() {
        assert_eq!(rfc3339(UNIX_EPOCH - Duration::from_secs(5)), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(at(59) + Duration::from_millis(999)), "1970-01-01T00:00:59Z");
    }

    #[test]
    fn the_time_of_day_is_the_end_of_the_stamp() {
        assert_eq!(time_of_day(at(1_793_536_496)), "12:34:56Z");
        assert_eq!(time_of_day(at(0)), "00:00:00Z");
    }
}
