use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{DateTime, Local};

use crate::frame::{CompletedFrame, FrameStore, ProjectName};
use crate::state::{OngoingFrame, StateStoreBackend};

/// An in-memory store implementation for testing purposes only.
/// Stores all data in instance variables without any persistence.
#[derive(Default)]
pub struct InMemoryStore {
    frames: RefCell<HashMap<String, CompletedFrame>>,
    ongoing_frame: RefCell<Option<OngoingFrame>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            frames: RefCell::new(HashMap::new()),
            ongoing_frame: RefCell::new(None),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InMemoryStoreError {
    Generic(String),
}

impl std::fmt::Display for InMemoryStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InMemoryStoreError::Generic(msg) => write!(f, "InMemoryStore error: {}", msg),
        }
    }
}

impl FrameStore for InMemoryStore {
    type FrameStoreError = InMemoryStoreError;

    fn save_frame(&self, frame: &CompletedFrame) -> Result<(), Self::FrameStoreError> {
        let mut frames = self.frames.borrow_mut();
        frames.insert(frame.frame().id().to_string(), frame.clone());
        Ok(())
    }

    fn get_projects(&self) -> Result<Vec<ProjectName>, Self::FrameStoreError> {
        let frames = self.frames.borrow();
        let mut projects: Vec<ProjectName> = frames
            .values()
            .map(|frame| frame.frame().project().clone())
            .collect();
        projects.sort();
        projects.dedup();
        Ok(projects)
    }

    fn get_last_frame(&self) -> Option<CompletedFrame> {
        let frames = self.frames.borrow();
        frames.values().max_by_key(|frame| frame.end()).cloned()
    }

    fn get_frame(&self, frame_id: &str) -> Result<Option<CompletedFrame>, Self::FrameStoreError> {
        let frames = self.frames.borrow();
        Ok(frames.get(frame_id).cloned())
    }

    fn get_frames(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<CompletedFrame>, Self::FrameStoreError> {
        let frames = self.frames.borrow();
        let mut result: Vec<CompletedFrame> = frames
            .values()
            .filter(|frame| {
                let frame_start = frame.frame().start();
                let frame_end = frame.end();
                // Include frames that overlap with the requested time range
                frame_start < &end && &frame_end > &start
            })
            .cloned()
            .collect();
        result.sort();
        Ok(result)
    }
}

impl StateStoreBackend for InMemoryStore {
    type StateStoreBackendError = InMemoryStoreError;

    fn get(&self) -> Result<Option<OngoingFrame>, Self::StateStoreBackendError> {
        Ok(self.ongoing_frame.borrow().clone())
    }

    fn store(&self, state: &OngoingFrame) -> Result<(), Self::StateStoreBackendError> {
        *self.ongoing_frame.borrow_mut() = Some(state.clone());
        Ok(())
    }

    fn clear(&self) -> Result<bool, Self::StateStoreBackendError> {
        let had_frame = self.ongoing_frame.borrow().is_some();
        *self.ongoing_frame.borrow_mut() = None;
        Ok(had_frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::NonEmptyString;
    use crate::frame::Frame;
    use chrono::{TimeZone, Timelike};

    fn create_test_project() -> ProjectName {
        ProjectName::from(NonEmptyString::new("test_project").unwrap())
    }

    fn create_test_frame(project: ProjectName, start_hour: u32, end_hour: u32) -> CompletedFrame {
        let start = Local
            .with_ymd_and_hms(2025, 1, 1, start_hour, 0, 0)
            .unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 1, end_hour, 0, 0).unwrap();
        let frame = Frame::new(project, None, Some(start), Some(end), vec![], None);
        CompletedFrame::from_frame(frame).unwrap()
    }

    fn create_test_ongoing_frame(project: ProjectName) -> OngoingFrame {
        let start = Local.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        OngoingFrame::new(project, start, vec![])
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = InMemoryStore::new();
        assert!(store.get_last_frame().is_none());
        assert!(store.get().unwrap().is_none());
    }

    #[test]
    fn test_save_and_retrieve_frame() {
        let store = InMemoryStore::new();
        let project = create_test_project();
        let frame = create_test_frame(project, 9, 10);
        let frame_id = frame.frame().id().to_string();

        store.save_frame(&frame).unwrap();

        let retrieved = store.get_frame(&frame_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().frame().id(), frame_id);
    }

    #[test]
    fn test_get_last_frame() {
        let store = InMemoryStore::new();
        let project = create_test_project();

        let frame1 = create_test_frame(project.clone(), 9, 10);
        let frame2 = create_test_frame(project.clone(), 11, 12);
        let frame3 = create_test_frame(project, 10, 11);

        store.save_frame(&frame1).unwrap();
        store.save_frame(&frame2).unwrap();
        store.save_frame(&frame3).unwrap();

        let last = store.get_last_frame().unwrap();
        assert_eq!(last.frame().id(), frame2.frame().id());
    }

    #[test]
    fn test_get_projects() {
        let store = InMemoryStore::new();
        let project1 = ProjectName::from(NonEmptyString::new("project_a").unwrap());
        let project2 = ProjectName::from(NonEmptyString::new("project_b").unwrap());

        let frame1 = create_test_frame(project1.clone(), 9, 10);
        let frame2 = create_test_frame(project2.clone(), 10, 11);
        let frame3 = create_test_frame(project1.clone(), 11, 12);

        store.save_frame(&frame1).unwrap();
        store.save_frame(&frame2).unwrap();
        store.save_frame(&frame3).unwrap();

        let projects = store.get_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&project1));
        assert!(projects.contains(&project2));
    }

    #[test]
    fn test_get_frames_in_range() {
        let store = InMemoryStore::new();
        let project = create_test_project();

        let frame1 = create_test_frame(project.clone(), 8, 9); // Outside range
        let frame2 = create_test_frame(project.clone(), 9, 10); // In range
        let frame3 = create_test_frame(project.clone(), 10, 11); // In range
        let frame4 = create_test_frame(project, 12, 13); // Outside range

        store.save_frame(&frame1).unwrap();
        store.save_frame(&frame2).unwrap();
        store.save_frame(&frame3).unwrap();
        store.save_frame(&frame4).unwrap();

        let start = Local.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 1, 11, 30, 0).unwrap();

        let frames = store.get_frames(start, end).unwrap();
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn test_store_and_get_ongoing_frame() {
        let store = InMemoryStore::new();
        let project = create_test_project();
        let ongoing = create_test_ongoing_frame(project.clone());

        store.store(&ongoing).unwrap();

        let retrieved = store.get().unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.project(), &project);
    }

    #[test]
    fn test_clear_ongoing_frame() {
        let store = InMemoryStore::new();
        let project = create_test_project();
        let ongoing = create_test_ongoing_frame(project);

        store.store(&ongoing).unwrap();
        assert!(store.get().unwrap().is_some());

        let had_frame = store.clear().unwrap();
        assert!(had_frame);
        assert!(store.get().unwrap().is_none());
    }

    #[test]
    fn test_clear_when_no_ongoing_frame() {
        let store = InMemoryStore::new();
        let had_frame = store.clear().unwrap();
        assert!(!had_frame);
    }

    #[test]
    fn test_update_existing_frame() {
        let store = InMemoryStore::new();
        let project = create_test_project();
        let frame = create_test_frame(project.clone(), 9, 10);
        let frame_id = frame.frame().id().to_string();

        store.save_frame(&frame).unwrap();

        // Create a new frame with the same ID but different times
        let start = Local.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let updated_frame = Frame::new(
            project,
            Some(frame_id.clone()),
            Some(start),
            Some(end),
            vec![],
            None,
        );
        let updated_frame = CompletedFrame::from_frame(updated_frame).unwrap();

        store.save_frame(&updated_frame).unwrap();

        let retrieved = store.get_frame(&frame_id).unwrap().unwrap();
        assert_eq!(retrieved.end().hour(), 12);
    }
}
