use chrono::{DateTime, Local};

use crate::{
    common::NonEmptyString,
    frame::{CompletedFrame, Frame, FrameEdit, ProjectName},
};

/// The backend to store the state (i.e. ongoing frames)
/// This needs to be implemented by specific storage. To actually save and load state, use `StateStore`.
pub trait StateStoreBackend {
    type StateStoreBackendError;
    /// Get the currently ongoing frame, if there is one. Returns None if there is none
    fn get(&self) -> Result<Option<OngoingFrame>, Self::StateStoreBackendError>;
    /// Store an ongoing frame. Overwrites an existing ongoing frame.
    fn store(&self, state: &OngoingFrame) -> Result<(), Self::StateStoreBackendError>;
    /// Clear an ongoing frame. The returned boolean indicates whether there was an ongoing frame. True if there was, false if there was not.
    fn clear(&self) -> Result<bool, Self::StateStoreBackendError>;
}

pub trait TrackingState {}
pub struct Ongoing {}
pub struct Stopped {}
impl TrackingState for Ongoing {}
impl TrackingState for Stopped {}

// The state store is the wrapper around the StateStoreBackend to protect from invalid access.
// Type state pattern is used to enable and disable methods of the StateStore based on whether there is an
// ongoing frame or not.
pub struct StateStore<'a, S: StateStoreBackend, T: TrackingState> {
    backend: &'a S,
    marker: std::marker::PhantomData<T>,
}

impl<'a, S: StateStoreBackend, T: TrackingState> StateStore<'a, S, T> {
    fn new(backend: &'a S) -> Self {
        Self {
            backend,
            marker: std::marker::PhantomData,
        }
    }
}

pub struct FrameStopped<'a, S: StateStoreBackend> {
    pub frame: CompletedFrame,
    #[allow(dead_code)]
    // For API completness we want to include the store here even though it is currently unused.
    pub store: StateStore<'a, S, Stopped>,
}

impl<'a, S> StateStore<'a, S, Ongoing>
where
    S: StateStoreBackend,
{
    pub fn stop(
        self,
        at: &DateTime<Local>,
    ) -> Result<FrameStopped<'a, S>, S::StateStoreBackendError> {
        let frame = Frame::from(self.get_ongoing()?);
        let completed_frame = frame.set_end(at.clone());
        self.backend.clear()?;
        Ok(FrameStopped {
            frame: completed_frame,
            store: StateStore::new(self.backend),
        })
    }

    pub fn cancel(self) -> Result<(), S::StateStoreBackendError> {
        self.backend.clear()?;
        Ok(())
    }

    pub fn update_ongoing(
        &self,
        ongoing_frame: OngoingFrame,
    ) -> Result<(), S::StateStoreBackendError> {
        self.backend.store(&ongoing_frame)?;
        Ok(())
    }

    pub fn get_ongoing(&self) -> Result<OngoingFrame, S::StateStoreBackendError> {
        Ok(self
            .backend
            .get()?
            .expect("Ongoing StateStore does not have ongoing frame. This may not happen!"))
    }
}

pub struct FrameStarted<'a, S: StateStoreBackend> {
    pub frame: OngoingFrame,
    #[allow(dead_code)]
    // For API completness we want to include the store here even though it is currently unused.
    pub store: StateStore<'a, S, Ongoing>,
}

impl<'a, S> StateStore<'a, S, Stopped>
where
    S: StateStoreBackend,
{
    pub fn start(
        self,
        project: ProjectName,
        start: DateTime<Local>,
        tags: Vec<NonEmptyString>,
    ) -> Result<FrameStarted<'a, S>, S::StateStoreBackendError> {
        let ongoing_frame = OngoingFrame::new(project, start, tags);
        self.backend.store(&ongoing_frame)?;
        Ok(FrameStarted {
            frame: ongoing_frame,
            store: StateStore::new(self.backend),
        })
    }
}

pub enum StateStoreVariant<'a, S: StateStoreBackend> {
    Ongoing(StateStore<'a, S, Ongoing>),
    Stopped(StateStore<'a, S, Stopped>),
}

/// Getter for a StateStore, based on a backend.
/// Use this method to get the StateStore in the currently active state.
pub fn get_state_store<'a, S: StateStoreBackend>(
    backend: &'a S,
) -> Result<StateStoreVariant<'a, S>, S::StateStoreBackendError> {
    match backend.get()? {
        Some(_) => Ok(StateStoreVariant::Ongoing(StateStore::new(backend))),
        None => Ok(StateStoreVariant::Stopped(StateStore::new(backend))),
    }
}

#[derive(Debug)]
///Representation of a currently ongoing frame
/// The frame is not completed and the storing of this is delegated to the StateStoreBackend
pub struct OngoingFrame {
    project: ProjectName,
    start: DateTime<Local>,
    tags: Vec<NonEmptyString>,
}

impl OngoingFrame {
    pub fn new(project: ProjectName, start: DateTime<Local>, tags: Vec<NonEmptyString>) -> Self {
        Self {
            project,
            start,
            tags,
        }
    }

    pub fn project(&self) -> &ProjectName {
        &self.project
    }

    pub fn start(&self) -> &DateTime<Local> {
        &self.start
    }

    pub fn tags(&self) -> &[NonEmptyString] {
        &self.tags
    }

    pub fn update_from(&mut self, edit: FrameEdit) {
        self.project = edit.project().clone();
        self.start = edit.start();
        self.tags = Vec::from(edit.tags());
    }
}
