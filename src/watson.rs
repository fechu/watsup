// The compatiblity layer to watson (https://github.com/jazzband/Watson/)
//

use chrono::TimeZone;
use serde::{Deserialize, Serialize, ser::SerializeSeq};

use crate::{
    common::NonEmptyString,
    frame::{self},
};

#[derive(Clone)]
pub struct Frame {
    start_timestamp: i64,
    end_timestamp: i64,
    project: NonEmptyString,
    id: String,
    tags: Vec<NonEmptyString>,
    last_edit_timestamp: i64,
}

impl From<frame::CompletedFrame> for Frame {
    fn from(completed_frame: frame::CompletedFrame) -> Self {
        Self {
            start_timestamp: completed_frame.frame().start().timestamp(),
            end_timestamp: completed_frame.end().timestamp(),
            project: completed_frame.frame().project().clone(),
            id: completed_frame.frame().id().into(),
            tags: completed_frame.frame().tags().into(),
            last_edit_timestamp: completed_frame.frame().last_edit().timestamp(),
        }
    }
}

impl From<Frame> for frame::CompletedFrame {
    fn from(value: Frame) -> Self {
        Self::from_frame(frame::Frame::new(
            value.project,
            Some(value.id),
            chrono::Local
                .timestamp_opt(value.start_timestamp, 0)
                .earliest(),
            chrono::Local.timestamp_opt(value.end_timestamp, 0).latest(),
            value.tags,
            chrono::Local
                .timestamp_opt(value.last_edit_timestamp, 0)
                .latest(),
        ))
        .unwrap()
    }
}

impl From<&frame::CompletedFrame> for Frame {
    fn from(completed_frame: &frame::CompletedFrame) -> Self {
        Self::from(completed_frame.clone())
    }
}

// Custom serialization implemention to match the format of watson
impl Serialize for Frame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(6))?;
        seq.serialize_element(&self.start_timestamp)?;
        seq.serialize_element(&self.end_timestamp)?;
        seq.serialize_element(&self.project)?;
        seq.serialize_element(&self.id)?;
        seq.serialize_element(&self.tags)?;
        seq.serialize_element(&self.last_edit_timestamp)?;
        seq.end()
    }
}

// Custom deserialization implemention to match the format of watson
impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seq = <[serde_json::Value; 6]>::deserialize(deserializer)?;
        let mut iter = seq.into_iter();

        let start_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid start_timestamp"))?;
        let end_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid end_timestamp"))?;
        let project = iter
            .next()
            .ok_or_else(|| serde::de::Error::custom("Missing project"))?;
        let project: NonEmptyString =
            serde_json::from_value(project).map_err(serde::de::Error::custom)?;
        let id = iter
            .next()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| serde::de::Error::custom("Invalid id"))?;
        let tags = iter
            .next()
            .ok_or_else(|| serde::de::Error::custom("Missing tags"))?;
        let tags: Vec<NonEmptyString> =
            serde_json::from_value(tags).map_err(serde::de::Error::custom)?;
        let last_edit_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid last_edit_timestamp"))?;

        Ok(Frame {
            start_timestamp,
            end_timestamp,
            project,
            id,
            tags,
            last_edit_timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn make_test_frame() -> Frame {
        Frame {
            start_timestamp: 1620000000,
            end_timestamp: 1620003600,
            project: NonEmptyString::new("test_project").unwrap(),
            id: "abc123".to_string(),
            tags: vec![
                NonEmptyString::new("tag1").unwrap(),
                NonEmptyString::new("tag2").unwrap(),
            ],
            last_edit_timestamp: 1620004000,
        }
    }

    #[test]
    fn test_frame_serialization() {
        let frame = make_test_frame();
        let serialized = serde_json::to_string(&frame).unwrap();
        // Should be a JSON array of 6 elements
        let v: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert!(v.is_array());
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 6);
        assert_eq!(arr[0], 1620000000);
        assert_eq!(arr[1], 1620003600);
        assert_eq!(arr[2], "test_project");
        assert_eq!(arr[3], "abc123");
        assert_eq!(arr[4], serde_json::json!(["tag1", "tag2"]));
        assert_eq!(arr[5], 1620004000);
    }

    #[test]
    fn test_frame_deserialization() {
        let json = r#"
            [
                1620000000,
                1620003600,
                "test_project",
                "abc123",
                ["tag1", "tag2"],
                1620004000
            ]
        "#;
        let frame: Frame = serde_json::from_str(json).unwrap();
        assert_eq!(frame.start_timestamp, 1620000000);
        assert_eq!(frame.end_timestamp, 1620003600);
        assert_eq!(frame.project.to_string(), "test_project");
        assert_eq!(frame.id, "abc123");
        assert_eq!(frame.tags.len(), 2);
        assert_eq!(frame.tags[0].to_string(), "tag1");
        assert_eq!(frame.tags[1].to_string(), "tag2");
        assert_eq!(frame.last_edit_timestamp, 1620004000);
    }
}
