use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;

pub fn format_date(date: &str, format: &str, timezone: Tz) -> String {
    let date = if date.contains("T") {
        DateTime::parse_from_rfc3339(date)
            .unwrap()
            .with_timezone(&timezone)
    } else {
        NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(timezone)
            .unwrap()
    };

    date.format(format).to_string()
}
