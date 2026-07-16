use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::vec2f;
use winit::dpi::{LogicalPosition, LogicalSize};

use super::*;
use crate::CursorInfo;

/// Convenience for building a geometry with an explicit position and square
/// size, decoupled from `from_cursor_info`'s caret-offset math so the tracker
/// tests can use round numbers.
fn geometry_at(position_x: f32, position_y: f32, size: f32) -> ImeCandidateGeometry {
    ImeCandidateGeometry {
        position: LogicalPosition::new(position_x, position_y),
        size: LogicalSize::new(size, size),
    }
}

#[test]
fn geometry_from_cursor_info_offsets_below_caret() {
    let cursor_info = CursorInfo {
        position: RectF::new(vec2f(10., 20.), vec2f(100., 200.)),
        font_size: 16.,
    };
    let geometry = ImeCandidateGeometry::from_cursor_info(&cursor_info);
    // Anchored at the caret origin, pushed down by 1.2x the font size.
    assert_eq!(
        geometry.position,
        LogicalPosition::new(10., 20. + 1.2 * 16.)
    );
    // Square of the font size (ignored by winit's X11 backend, but forwarded).
    assert_eq!(geometry.size, LogicalSize::new(16., 16.));
}

#[test]
fn first_refresh_sets_position_without_nudging() {
    let mut tracker = ImeCandidatePositionTracker::new();
    let geometry = geometry_at(5., 7., 12.);

    let updates = tracker.refresh(geometry);

    // No prior position, so a single direct set is enough — no nudge.
    assert_eq!(updates, vec![geometry]);
}

#[test]
fn unchanged_position_nudges_and_restores() {
    let mut tracker = ImeCandidatePositionTracker::new();
    let geometry = geometry_at(5., 7., 12.);
    tracker.refresh(geometry);

    let updates = tracker.refresh(geometry);

    // Same position as last time: nudge by one pixel on y, then restore.
    let nudged = geometry_at(5., 8., 12.);
    assert_eq!(updates, vec![nudged, geometry]);
}

#[test]
fn changed_position_sets_directly_without_nudging() {
    let mut tracker = ImeCandidatePositionTracker::new();
    tracker.refresh(geometry_at(5., 7., 12.));

    let moved = geometry_at(50., 70., 12.);
    let updates = tracker.refresh(moved);

    // The caret moved, so winit's cache is already stale — set directly.
    assert_eq!(updates, vec![moved]);
}

#[test]
fn nudge_only_offsets_y_by_one_pixel() {
    let mut tracker = ImeCandidatePositionTracker::new();
    let geometry = geometry_at(5., 7., 12.);
    tracker.refresh(geometry);

    let updates = tracker.refresh(geometry);
    assert_eq!(updates.len(), 2);
    let nudged = updates[0];
    let restored = updates[1];

    // The nudge moves only y by +1; x and size are unchanged.
    assert_eq!(nudged.position, LogicalPosition::new(5., 8.));
    assert_eq!(nudged.size, geometry.size);
    // The restore brings the position back to the requested one.
    assert_eq!(restored, geometry);
}

#[test]
fn tracker_compares_against_last_effective_position() {
    let mut tracker = ImeCandidatePositionTracker::new();
    let first = geometry_at(5., 7., 12.);
    let second = geometry_at(50., 70., 12.);

    // First request positions directly.
    assert_eq!(tracker.refresh(first), vec![first]);
    // Same position again: nudge then restore around `first`.
    assert_eq!(
        tracker.refresh(first),
        vec![geometry_at(5., 8., 12.), first]
    );
    // Different position: a single direct set, no nudge.
    assert_eq!(tracker.refresh(second), vec![second]);
    // The tracker remembers the restore target (`second`), not any transient
    // nudge, so repeating `second` nudges around `second` rather than `first`.
    assert_eq!(
        tracker.refresh(second),
        vec![geometry_at(50., 71., 12.), second]
    );
}
