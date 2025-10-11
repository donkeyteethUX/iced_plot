use std::sync::{Arc, RwLock};

/// Represents a shared axis link that can synchronize camera positions
/// across multiple plot widgets.
#[derive(Clone, Debug)]
pub struct AxisLink {
    inner: Arc<RwLock<AxisLinkInner>>,
}

#[derive(Debug)]
struct AxisLinkInner {
    /// The shared camera position
    position: f64,
    /// The shared camera half extent
    half_extent: f64,
    /// Version counter to detect changes
    version: u64,
}

impl AxisLink {
    /// Create a new axis link with initial position and half extent
    pub fn new(position: f64, half_extent: f64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(AxisLinkInner {
                position,
                half_extent,
                version: 0,
            })),
        }
    }

    /// Get the current position and half extent
    pub(crate) fn get(&self) -> (f64, f64, u64) {
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

    /// Get the current version
    pub(crate) fn version(&self) -> u64 {
        self.inner.read().unwrap().version
    }
}

impl Default for AxisLink {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}
