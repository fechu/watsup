use chrono::{Local, NaiveDateTime, NaiveTime, TimeZone};

/// Variants for parsing a date, time or datetime argument from the command line.
/// See `parse_datetime` for usage
pub enum DateTimeArgument {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
}

/// Parse a datetime string into a `chrono::DateTime<Local>`
///
/// Accepts formats "YYYY-MM-DD HH:MM" or "HH:MM"
pub fn parse_datetime_options(arg: &str) -> Result<DateTimeArgument, String> {
    let arg = arg.trim();
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(arg, "%Y-%m-%d %H:%M") {
        Ok(DateTimeArgument::DateTime(dt))
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(arg, "%Y-%m-%d") {
        Ok(DateTimeArgument::Date(date))
    } else if let Ok(time) = chrono::NaiveTime::parse_from_str(arg, "%H:%M") {
        Ok(DateTimeArgument::Time(time))
    } else {
        Err("Invalid datetime expected format YYYY-MM-DD HH:MM or HH:MM".to_string())
    }
}

pub fn parse_datetime(
    arg: &str,
    default_date: chrono::NaiveDate,
    default_time: chrono::NaiveTime,
) -> Result<chrono::DateTime<Local>, String> {
    let local_datetime = match parse_datetime_options(arg)? {
        DateTimeArgument::DateTime(naive_date_time) => naive_date_time,
        DateTimeArgument::Date(naive_date) => NaiveDateTime::new(naive_date, default_time),
        DateTimeArgument::Time(naive_time) => NaiveDateTime::new(default_date, naive_time),
    };
    Local
        .from_local_datetime(&local_datetime)
        .single()
        .ok_or("Datetime is not valid in local timezone".to_string())
}

/// Parse a date or time or datetime
/// If any part is missing is it filled with the current date or time
pub fn parse_datetime_now(arg: &str) -> Result<chrono::DateTime<Local>, String> {
    let now = Local::now();
    parse_datetime(arg, now.date_naive(), now.time())
}

/// Parse a date that defaults to the beginning of the current day
/// By default if the time is not provided, the time will be set to 00:00 to include frames
/// from the very beginning of the day
pub fn parse_beginning_of_day(arg: &str) -> Result<chrono::DateTime<Local>, String> {
    parse_datetime(
        arg,
        Local::now().date_naive(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    )
}

/// Parse a date that defaults to the end of the current day
/// By default if the time is not provided, the time will be set to 23:59 to include frames
/// until the very end of the day
pub fn parse_end_of_day(arg: &str) -> Result<chrono::DateTime<Local>, String> {
    parse_datetime(
        arg,
        Local::now().date_naive(),
        NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use super::*;

    #[test]
    fn parse_full_datetime() {
        let datetime_str = "2025-01-02 11:12";
        let datetime = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 01, 02).unwrap(),
            NaiveTime::from_hms_opt(11, 12, 00).unwrap(),
        );
        match parse_datetime_options(datetime_str) {
            Ok(DateTimeArgument::DateTime(dt)) => {
                assert_eq!(datetime, dt);
            }
            Ok(_) => {
                panic!("Wrong parsing, expected DateTimeArgument::DateTime")
            }
            Err(e) => panic!("Failed to parse: {}", e),
        }
    }

    #[test]
    fn parse_time() {
        let time_str = "11:12";
        let time = NaiveTime::from_hms_opt(11, 12, 00).unwrap();
        match parse_datetime_options(time_str) {
            Ok(DateTimeArgument::Time(t)) => {
                assert_eq!(time, t);
            }
            Ok(_) => {
                panic!("Wrong parsing, expected DateTimeArgument::Time")
            }
            Err(e) => panic!("Failed to parse: {}", e),
        }
    }

    #[test]
    fn parse_date() {
        let date_str = "2022-01-02";
        let date = NaiveDate::from_ymd_opt(2022, 1, 2).unwrap();
        match parse_datetime_options(date_str) {
            Ok(DateTimeArgument::Date(d)) => {
                assert_eq!(date, d);
            }
            Ok(_) => {
                panic!("Wrong parsing, expected DateTimeArgument::Date")
            }
            Err(e) => panic!("Failed to parse: {}", e),
        }
    }

    #[test]
    fn parse_beginning_of_day_sets_time_to_midnight_when_time_missing() {
        let date_str = "2023-05-10";
        let expected_date = NaiveDate::from_ymd_opt(2023, 5, 10).unwrap();
        let expected_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        let dt = parse_beginning_of_day(date_str).expect("Should parse beginning of day");
        assert_eq!(dt.date_naive(), expected_date);
        assert_eq!(dt.time(), expected_time);
    }

    #[test]
    fn parse_beginning_of_day_preserves_time_when_provided() {
        let datetime_str = "2023-05-10 08:30";
        let expected_date = NaiveDate::from_ymd_opt(2023, 5, 10).unwrap();
        let expected_time = NaiveTime::from_hms_opt(8, 30, 0).unwrap();

        let dt = parse_beginning_of_day(datetime_str).expect("Should parse beginning of day");
        assert_eq!(dt.date_naive(), expected_date);
        assert_eq!(dt.time(), expected_time);
    }

    #[test]
    fn parse_end_of_day_sets_time_to_end_when_time_missing() {
        let date_str = "2023-05-10";
        let expected_date = NaiveDate::from_ymd_opt(2023, 5, 10).unwrap();
        let expected_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

        let dt = parse_end_of_day(date_str).expect("Should parse end of day");
        assert_eq!(dt.date_naive(), expected_date);
        assert_eq!(dt.time(), expected_time);
    }

    #[test]
    fn parse_end_of_day_preserves_time_when_provided() {
        let datetime_str = "2023-05-10 21:45";
        let expected_date = NaiveDate::from_ymd_opt(2023, 5, 10).unwrap();
        let expected_time = NaiveTime::from_hms_opt(21, 45, 0).unwrap();

        let dt = parse_end_of_day(datetime_str).expect("Should parse end of day");
        assert_eq!(dt.date_naive(), expected_date);
        assert_eq!(dt.time(), expected_time);
    }
}
