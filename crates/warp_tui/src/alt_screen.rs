//! Full-frame alternate-screen rendering and input forwarding for the TUI.

use std::sync::Arc;

use parking_lot::FairMutex;
use warp::tui_export::TerminalModel;
use warp_terminal::model::escape_sequences::{
    EscCodes, KeystrokeWithDetails, ToEscapeSequence as _, C0, C1,
};
use warp_terminal::model::grid::Dimensions as _;
use warp_terminal::model::mouse::{MouseAction, MouseButton, MouseState};
use warp_terminal::model::{Point, TermMode};
use warpui_core::elements::tui::{
    TuiBuffer, TuiConstraint, TuiElement, TuiEvent, TuiEventContext, TuiLayoutContext,
    TuiPaintContext, TuiRect, TuiRectExt as _, TuiSize,
};
use warpui_core::AppContext;

use crate::terminal_block::{cell_to_style, sanitized_symbol};
use crate::terminal_session_view::TuiTerminalSessionAction;

/// Paints and owns input for the active terminal alternate screen.
pub(super) struct TuiAltScreenElement {
    model: Arc<FairMutex<TerminalModel>>,
}

impl TuiAltScreenElement {
    pub(super) fn new(model: Arc<FairMutex<TerminalModel>>) -> Self {
        Self { model }
    }

    fn event_bytes(&self, event: &TuiEvent, area: TuiRect) -> Option<Vec<u8>> {
        let model = self.model.lock();
        if !model.is_alt_screen_active() {
            return None;
        }

        match event {
            TuiEvent::KeyDown {
                keystroke,
                chars,
                details,
                ..
            } => KeystrokeWithDetails {
                keystroke,
                key_without_modifiers: details.key_without_modifiers.as_deref(),
                chars: Some(chars),
            }
            .to_escape_sequence(&*model)
            .or_else(|| named_key_bytes(keystroke))
            .or_else(|| (!chars.is_empty()).then(|| chars.as_bytes().to_vec())),
            _ => mouse_event_bytes(event, area, &model),
        }
    }
}

impl TuiElement for TuiAltScreenElement {
    fn layout(
        &mut self,
        constraint: TuiConstraint,
        _ctx: &mut TuiLayoutContext,
        _app: &AppContext,
    ) -> TuiSize {
        constraint.max
    }

    fn render(&self, area: TuiRect, buffer: &mut TuiBuffer, _ctx: &mut TuiPaintContext) {
        let model = self.model.lock();
        if !model.is_alt_screen_active() {
            return;
        }

        let colors = model.colors();
        let grid = model.alt_screen().grid_handler();
        let row_count = grid.visible_rows().min(usize::from(area.height));
        let column_count = grid.columns().min(usize::from(area.width));
        for row_index in 0..row_count {
            let Some(row) = grid.row(row_index) else {
                continue;
            };
            let y = area.y.saturating_add(row_index as u16);
            for column_index in 0..column_count {
                let cell = &row[column_index];
                let x = area.x.saturating_add(column_index as u16);
                if let Some(buffer_cell) = buffer.cell_mut((x, y)) {
                    buffer_cell
                        .set_symbol(&sanitized_symbol(cell))
                        .set_style(cell_to_style(cell, &colors));
                }
            }
        }
    }

    fn cursor_position(&self, area: TuiRect, _ctx: &mut TuiPaintContext) -> Option<(u16, u16)> {
        let model = self.model.lock();
        if !model.is_alt_screen_active() || !model.is_term_mode_set(TermMode::SHOW_CURSOR) {
            return None;
        }
        let cursor = model.alt_screen().grid_handler().cursor_render_point();
        let column = u16::try_from(cursor.col).ok()?;
        let row = u16::try_from(cursor.row).ok()?;
        (column < area.width && row < area.height).then_some((column, row))
    }

    fn dispatch_event(
        &mut self,
        event: &TuiEvent,
        area: TuiRect,
        event_ctx: &mut TuiEventContext,
        _ctx: &mut TuiLayoutContext,
        _app: &AppContext,
    ) -> bool {
        let Some(bytes) = self.event_bytes(event, area) else {
            return false;
        };
        event_ctx.dispatch_typed_action(TuiTerminalSessionAction::ForwardAltScreenInput(bytes));
        true
    }
}

fn named_key_bytes(keystroke: &warpui_core::keymap::Keystroke) -> Option<Vec<u8>> {
    match keystroke.key.as_str() {
        "enter" => Some(vec![C0::CR]),
        "tab" | "\t" if keystroke.shift => Some([C1::CSI, b"Z"].concat()),
        "tab" | "\t" => Some(vec![C0::HT]),
        "escape" => Some(vec![C0::ESC]),
        "insert" => Some(b"\x1b[2~".to_vec()),
        "delete" => Some(b"\x1b[3~".to_vec()),
        "pageup" => Some(b"\x1b[5~".to_vec()),
        "pagedown" => Some(b"\x1b[6~".to_vec()),
        _ => None,
    }
}
fn mouse_event_bytes(event: &TuiEvent, area: TuiRect, model: &TerminalModel) -> Option<Vec<u8>> {
    let position = event.position()?;
    if !area.contains_point(position) {
        return None;
    }

    if let TuiEvent::ScrollWheel { delta, .. } = event {
        if !mouse_reporting_enabled(model) {
            return model
                .is_term_mode_set(TermMode::ALTERNATE_SCROLL)
                .then(|| alternate_scroll_bytes(delta.1))
                .filter(|bytes| !bytes.is_empty());
        }
    } else if !mouse_reporting_enabled(model) {
        return None;
    }

    let point = terminal_point(position, area, model)?;
    let mouse = match event {
        TuiEvent::ScrollWheel {
            delta, modifiers, ..
        } => MouseState::new(
            MouseButton::Wheel,
            MouseAction::Scrolled {
                delta: saturating_i32(delta.1),
            },
            *modifiers,
        ),
        TuiEvent::LeftMouseDown { modifiers, .. } => {
            MouseState::new(MouseButton::Left, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::LeftMouseUp { modifiers, .. } => {
            MouseState::new(MouseButton::Left, MouseAction::Released, *modifiers)
        }
        TuiEvent::LeftMouseDragged { modifiers, .. } if drag_reporting_enabled(model) => {
            MouseState::new(MouseButton::LeftDrag, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::MiddleMouseDown { modifiers, .. } => {
            MouseState::new(MouseButton::Middle, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::MiddleMouseUp { modifiers, .. } => {
            MouseState::new(MouseButton::Middle, MouseAction::Released, *modifiers)
        }
        TuiEvent::MiddleMouseDragged { modifiers, .. } if drag_reporting_enabled(model) => {
            MouseState::new(MouseButton::MiddleDrag, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::RightMouseDown { modifiers, .. } => {
            MouseState::new(MouseButton::Right, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::RightMouseUp { modifiers, .. } => {
            MouseState::new(MouseButton::Right, MouseAction::Released, *modifiers)
        }
        TuiEvent::RightMouseDragged { modifiers, .. } if drag_reporting_enabled(model) => {
            MouseState::new(MouseButton::RightDrag, MouseAction::Pressed, *modifiers)
        }
        TuiEvent::MouseMoved { modifiers, .. }
            if model.is_term_mode_set(TermMode::MOUSE_MOTION) =>
        {
            MouseState::new(MouseButton::Move, MouseAction::Pressed, *modifiers)
        }
        _ => return None,
    }
    .set_point(point);
    mouse.to_escape_sequence(model)
}

fn mouse_reporting_enabled(model: &TerminalModel) -> bool {
    model.is_term_mode_set(TermMode::SGR_MOUSE) && model.is_term_mode_set(TermMode::MOUSE_MODE)
}

fn drag_reporting_enabled(model: &TerminalModel) -> bool {
    model.is_term_mode_set(TermMode::MOUSE_DRAG) || model.is_term_mode_set(TermMode::MOUSE_MOTION)
}

fn terminal_point(
    position: warpui_core::elements::tui::TuiPoint,
    area: TuiRect,
    model: &TerminalModel,
) -> Option<Point> {
    let grid = model.alt_screen().grid_handler();
    let max_column = grid.columns().checked_sub(1)?;
    let max_row = grid.visible_rows().checked_sub(1)?;
    Some(Point::new(
        usize::from(position.y.saturating_sub(area.y)).min(max_row),
        usize::from(position.x.saturating_sub(area.x)).min(max_column),
    ))
}

fn alternate_scroll_bytes(rows: isize) -> Vec<u8> {
    let command = if rows > 0 {
        EscCodes::ARROW_UP
    } else {
        EscCodes::ARROW_DOWN
    };
    let sequence = EscCodes::build_escape_sequence_with_c1(C1::SS3, &[command]);
    sequence.repeat(rows.unsigned_abs())
}

fn saturating_i32(value: isize) -> i32 {
    i32::try_from(value).unwrap_or(if value.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}

#[cfg(test)]
#[path = "alt_screen_tests.rs"]
mod tests;
