use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, Local, NaiveTime};

use crate::frame::CompletedFrame;

pub struct FrameLog<'a> {
    frames: &'a [CompletedFrame],
}

impl<'a> FrameLog<'a> {
    /// Create a new log given a start and an end date
    pub fn new(frames: &'a [CompletedFrame]) -> Self {
        FrameLog { frames }
    }

    /// Get the frames in this log grouped by day.
    /// The returned hashmap will only contain keys (days) where there is at least one frame in that day
    /// A frame is placed in the group of day A if the start date of the frame is on day A.
    fn grouped_by_day(&self) -> HashMap<DateTime<Local>, Vec<&'a CompletedFrame>> {
        let mut map = HashMap::new();
        let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        for frame in self.frames {
            let key = frame.frame().start().with_time(time).unwrap();
            map.entry(key).or_insert_with(Vec::new).push(frame);
        }
        map
    }
}

impl<'a> Display for FrameLog<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let grouped_by_day = self.grouped_by_day();
        let mut days = grouped_by_day.keys().collect::<Vec<_>>();
        days.sort_by(|a, b| b.cmp(a));
        for day in days {
            let frames = grouped_by_day.get(day).unwrap();
            let total_duration = frames.iter().map(|f| f.duration()).reduce(|f1, f2| f1 + f2);
            writeln!(f, "")?;
            writeln!(
                f,
                "{} ({}h {}min {}s)",
                day.format("%A %d %B %Y"),
                total_duration.unwrap().num_hours(),
                total_duration.unwrap().num_minutes() - total_duration.unwrap().num_hours() * 60,
                total_duration.unwrap().num_seconds() - total_duration.unwrap().num_minutes() * 60,
            )?;
            for frame in frames {
                writeln!(f, "  {}", frame)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod log_tests {
    use chrono::{Datelike, Duration, Timelike};

    use crate::{common::NonEmptyString, frame::Frame};

    use super::*;

    fn create_test_frame(start: DateTime<Local>, end: Option<DateTime<Local>>) -> CompletedFrame {
        let end_time = end.unwrap_or(start + Duration::minutes(15));
        CompletedFrame::from_frame(Frame::new(
            NonEmptyString::new("project name").unwrap(),
            None,
            Some(start),
            Some(end_time),
            vec![],
            None,
        ))
        .unwrap()
    }

    fn assert_same_day(date1: &DateTime<Local>, date2: &DateTime<Local>) {
        assert_eq!(date1.year(), date2.year());
        assert_eq!(date1.month(), date2.month());
        assert_eq!(date1.day(), date2.day());
    }

    #[test]
    fn test_empty_log_creates_empty_grouped_log() {
        let log = FrameLog::new(&[]);
        let grouped = log.grouped_by_day();

        assert_eq!(grouped.len(), 0);
    }

    #[test]
    fn test_single_frame_log_to_grouped_log() {
        let date = Local::now();
        let frame = create_test_frame(date, None);
        let frames = [frame];
        let log = FrameLog::new(&frames);
        let grouped = log.grouped_by_day();

        assert_eq!(grouped.len(), 1);
        let key = grouped.keys().next().unwrap();
        assert_same_day(key, &date);
        assert_eq!(key.hour(), 0);
        assert_eq!(key.minute(), 0);
        assert_eq!(key.second(), 0);
    }

    #[test]
    fn test_grouped_by_day() {
        use chrono::{Local, TimeZone};

        // Create three frames on two different days
        let start_time1 = Local.with_ymd_and_hms(2025, 11, 22, 10, 0, 0).unwrap();
        let start_time2 = Local.with_ymd_and_hms(2025, 11, 23, 10, 0, 0).unwrap();

        let frames = vec![
            create_test_frame(start_time1, None),
            create_test_frame(start_time2, None),
        ];

        let log = FrameLog::new(&frames);
        let grouped = log.grouped_by_day();

        assert_eq!(grouped.len(), 2);

        let key1 = Local.with_ymd_and_hms(2025, 11, 22, 0, 0, 0).unwrap();
        let key2 = Local.with_ymd_and_hms(2025, 11, 23, 0, 0, 0).unwrap();

        assert!(grouped.get(&key1).unwrap()[0] == &frames[0]);
        assert!(grouped.get(&key2).unwrap()[0] == &frames[1]);
    }

    #[test]
    fn test_frame_across_midnight_is_in_starting_day_group() {
        use chrono::{Local, TimeZone};

        // Create two frames on the same day but with a time difference of 24 hours
        let start_time = Local.with_ymd_and_hms(2025, 11, 22, 23, 45, 0).unwrap();
        let end_date = Local.with_ymd_and_hms(2025, 11, 23, 00, 15, 0).unwrap();

        let frames = vec![create_test_frame(start_time, Some(end_date))];

        let log = FrameLog::new(&frames);
        let grouped = log.grouped_by_day();

        assert_eq!(grouped.len(), 1);

        let key = grouped.keys().next().unwrap();
        assert_same_day(key, &start_time);
    }
}
