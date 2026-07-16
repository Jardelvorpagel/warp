//! Positions winit's IME candidate window and works around its position cache.
//!
//! winit caches the most recently requested IME candidate position and skips
//! repositioning when a subsequent request compares equal to the cached value.
//! That breaks when the window itself moves, resizes, or changes scale factor:
//! the candidate window is positioned relative to the window, so even though
//! the requested logical position is unchanged its on-screen location is now
//! stale. This module computes the candidate-window geometry from the active
//! caret and emits a deterministic sequence of `set_ime_position` requests that
//! forces winit to reposition only when actually necessary.

use winit::dpi::{LogicalPosition, LogicalSize};

use crate::CursorInfo;

/// The logical position and size we ask winit to place the IME candidate window
/// at, derived from the active text caret.
///
/// The candidate window is anchored at the caret's origin and pushed down by
/// ~1.2× the font size so it appears just beneath the line of text being
/// composed. The `size` field is a square of the font size; winit's X11 backend
/// ignores the size argument, but we compute and forward it on every platform so
/// the request shape stays consistent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImeCandidateGeometry {
    pub(crate) position: LogicalPosition<f32>,
    pub(crate) size: LogicalSize<f32>,
}

impl ImeCandidateGeometry {
    /// Builds the candidate-window geometry for the given active cursor info.
    pub(crate) fn from_cursor_info(cursor_info: &CursorInfo) -> Self {
        let position = LogicalPosition::new(
            cursor_info.position.origin_x(),
            cursor_info.position.origin_y() + (1.2 * cursor_info.font_size),
        );
        // The size argument is not supported on winit's X11 backend, but we
        // calculate it here anyway so the request is identical across platforms.
        let size = LogicalSize::new(cursor_info.font_size, cursor_info.font_size);
        Self { position, size }
    }

    /// Returns `self` offset by one logical pixel on the y axis, leaving the
    /// position's x and the size untouched. Used to invalidate winit's cached
    /// position before restoring the real one.
    fn nudged_down_by_one_pixel(&self) -> Self {
        Self {
            position: LogicalPosition::new(self.position.x, self.position.y + 1.),
            size: self.size,
        }
    }
}

/// Tracks the last IME candidate position we handed to winit and decides what
/// `set_ime_position` calls are needed to (re)position the candidate window.
///
/// [`ImeCandidatePositionTracker::refresh`] returns one of two update sequences:
/// - `[geometry]` on the first request, or when the requested position has
///   changed (the caret moved). winit's cache is already stale relative to the
///   new value, so a single request repositions the candidate window.
/// - `[nudge, geometry]` when the requested position is unchanged. winit would
///   otherwise treat the request as a no-op, so `nudge` (the geometry offset by
///   one pixel on the y axis) forces the cache to invalidate before `geometry`
///   restores the real position.
///
/// Applying the nudge only when the position is unchanged avoids the flicker of
/// nudging on every caret move while still repositioning after window
/// move/resize/scale-factor changes.
#[derive(Debug, Default)]
pub(crate) struct ImeCandidatePositionTracker {
    /// The last position we left the candidate window at — i.e. the restore
    /// target of the most recent sequence, never a transient nudge.
    last_position: Option<LogicalPosition<f32>>,
}

impl ImeCandidatePositionTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns the ordered sequence of candidate-window geometries to pass to
    /// winit's `set_ime_position` in order to place the candidate window at
    /// `geometry`, given what we last requested.
    pub(crate) fn refresh(&mut self, geometry: ImeCandidateGeometry) -> Vec<ImeCandidateGeometry> {
        let updates = match self.last_position {
            // First request: winit has nothing cached, so a single set positions
            // the candidate window.
            None => vec![geometry],
            // The requested logical position is unchanged, but the window may
            // have moved, resized, or changed scale factor since we last set it.
            // Nudge by one pixel and immediately restore to force winit to
            // invalidate its cache and reposition.
            Some(last) if last == geometry.position => {
                vec![geometry.nudged_down_by_one_pixel(), geometry]
            }
            // The caret moved, so winit's cache is already stale relative to the
            // new position — a single set is enough, and nudging here would only
            // cause unnecessary flicker.
            Some(_) => vec![geometry],
        };
        // Every sequence leaves the candidate window at `geometry.position`.
        self.last_position = Some(geometry.position);
        updates
    }
}

#[cfg(test)]
#[path = "ime_tests.rs"]
mod tests;
