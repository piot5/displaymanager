//! Shared geometric overlap detection for display topologies.
//!
//! This module provides a single, canonical implementation of the overlap
//! validation logic used by all platform backends (Windows CCD, Linux
//! WlrTopology, KdeTopology, UdevTopology). Extracting this into one
//! place ensures the validation behaviour is identical across platforms
//! and simplifies unit testing.
//!
//! # Design
//!
//! The function [`check_overlap`] operates on any slice of objects that
//! expose an `enabled` flag and a [`Rect`](crate::types::Rect) geometry
//! (origin + size). This is intentionally generic so that multiple backend
//! state types can be validated without converting to [`OutputState`] first.
//!
//! # Usage
//!
//! ```rust,ignore
//! use df_displmgr::backends::overlap::check_overlap;
//! use df_displmgr::types::{Rect, Point2D, Extent2D};
//!
//! struct MyOutput { enabled: bool, geometry: Rect }
//!
//! let outputs = vec![
//!     MyOutput { enabled: true,  geometry: Rect { origin: Point2D { x: 0, y: 0 }, size: Extent2D { width: 1920, height: 1080 } } },
//!     MyOutput { enabled: true,  geometry: Rect { origin: Point2D { x: 1920, y: 0 }, size: Extent2D { width: 1920, height: 1080 } } },
//! ];
//! assert!(check_overlap(&outputs).is_ok());
//! ```

use crate::error::DisplayResult;
use crate::types::Rect;

/// Trait for types that can be validated for geometric overlap.
///
/// Implemented by internal backend state types so that [`check_overlap`]
/// works without converting to [`OutputState`](crate::types::OutputState).
pub trait OverlapCheckable {
    /// Returns `true` if the output is currently enabled.
    fn is_enabled(&self) -> bool;
    /// Returns the geometric boundary of the output.
    fn geometry(&self) -> Rect;
}

/// Performs a pairwise geometric overlap check across all outputs.
///
/// Returns `Ok(())` if no two enabled outputs overlap. Returns
/// `Err(DisplayError::ConfigurationRejected)` with a description
/// of the first detected overlap.
///
/// # Type Parameters
///
/// * `T` — Any type implementing [`OverlapCheckable`]. This allows
///   backends to use their own internal state types.
///
/// # Errors
///
/// Returns [`DisplayError::ConfigurationRejected`](crate::error::DisplayError::ConfigurationRejected)
/// if any two enabled outputs overlap in the virtual desktop coordinate space.
pub fn check_overlap<T: OverlapCheckable>(outputs: &[T]) -> DisplayResult<()> {
    for (i, out_a) in outputs.iter().enumerate() {
        for out_b in outputs.iter().skip(i + 1) {
            if out_a.is_enabled() && out_b.is_enabled() {
                let a = out_a.geometry();
                let b = out_b.geometry();
                let a_x2 = a.origin.x + a.size.width as i32;
                let a_y2 = a.origin.y + a.size.height as i32;
                let b_x2 = b.origin.x + b.size.width as i32;
                let b_y2 = b.origin.y + b.size.height as i32;

                let overlap_x = a.origin.x < b_x2 && a_x2 > b.origin.x;
                let overlap_y = a.origin.y < b_y2 && a_y2 > b.origin.y;

                if overlap_x && overlap_y {
                    return Err(crate::error::DisplayError::ConfigurationRejected);
                }
            }
        }
    }
    Ok(())
}

// ── Blanket implementation for slices of (bool, Rect) tuples ──
impl OverlapCheckable for (bool, Rect) {
    fn is_enabled(&self) -> bool {
        self.0
    }
    fn geometry(&self) -> Rect {
        self.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Extent2D, Point2D};

    #[test]
    fn no_overlap() {
        let outputs = vec![
            (
                true,
                Rect {
                    origin: Point2D { x: 0, y: 0 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
            (
                true,
                Rect {
                    origin: Point2D { x: 1920, y: 0 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
        ];
        assert!(check_overlap(&outputs).is_ok());
    }

    #[test]
    fn overlapping_detected() {
        let outputs = vec![
            (
                true,
                Rect {
                    origin: Point2D { x: 0, y: 0 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
            (
                true,
                Rect {
                    origin: Point2D { x: 500, y: 500 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
        ];
        assert!(check_overlap(&outputs).is_err());
    }

    #[test]
    fn disabled_ignored() {
        let outputs = vec![
            (
                true,
                Rect {
                    origin: Point2D { x: 0, y: 0 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
            (
                false,
                Rect {
                    origin: Point2D { x: 0, y: 0 },
                    size: Extent2D {
                        width: 1920,
                        height: 1080,
                    },
                },
            ),
        ];
        assert!(check_overlap(&outputs).is_ok());
    }

    #[test]
    fn empty_slice() {
        let outputs: Vec<(bool, Rect)> = vec![];
        assert!(check_overlap(&outputs).is_ok());
    }
}
