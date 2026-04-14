use std::sync::{Arc, RwLock};

/// Represents a shared axis link that can synchronize camera positions
/// across multiple plot widgets.
#[derive(Clone, Debug, Default)]
pub struct AxisLink {
    inner: Arc<RwLock<AxisLinkInner>>,
}

#[derive(Debug, Default)]
struct AxisLinkInner {
    /// The shared camera position
    position: f64,
    /// The shared camera half extent
    half_extent: f64,
    /// Version counter to detect changes
    version: u64,
}

impl AxisLink {
    /// Create a new axis link.
    pub fn new() -> Self {
        Default::default()
    }

    /// Get the current position, half-extent, and version counter.
    ///
    /// Returns `(center, half_extent, version)`. The data-space axis range
    /// covered by the linked plots is `[center - half_extent, center + half_extent]`.
    ///
    /// The version counter is bumped on every camera change; downstream
    /// code can poll it to detect viewport updates without plumbing the
    /// `PlotUiMessage::RenderUpdate` stream.
    ///
    /// # Panics
    ///
    /// Panics only if the internal `RwLock` has been poisoned by a prior
    /// panic on another thread while holding the write lock. Not expected
    /// in normal operation.
    #[must_use]
    pub fn get(&self) -> (f64, f64, u64) {
        let inner = self.inner.read().unwrap();
        (inner.position, inner.half_extent, inner.version)
    }

    /// Update the position and half extent, incrementing the version
    pub(crate) fn set(&self, position: f64, half_extent: f64) {
        let mut inner = self.inner.write().unwrap();
        inner.position = position;
        inner.half_extent = half_extent;
        inner.version = inner.version.wrapping_add(1);
    }

    /// Get the current version counter.
    ///
    /// The counter is bumped on every camera change. A cheap way to detect
    /// whether the viewport has moved since the last observation without
    /// locking the value state.
    ///
    /// # Panics
    ///
    /// Panics only if the internal `RwLock` has been poisoned by a prior
    /// panic on another thread while holding the write lock. Not expected
    /// in normal operation.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.inner.read().unwrap().version
    }
}
