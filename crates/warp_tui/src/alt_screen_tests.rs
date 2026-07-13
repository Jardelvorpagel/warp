use std::sync::Arc;

use parking_lot::FairMutex;
use warp::terminal::model::ansi::{Handler as _, Mode};
use warp::tui_export::TerminalModel;
use warpui::event::{KeyEventDetails, ModifiersState};
use warpui::{App, EntityIdMap};
use warpui_core::elements::tui::{
    TuiBuffer, TuiBufferExt, TuiConstraint, TuiElement, TuiEvent, TuiLayoutContext,
    TuiPaintContext, TuiPoint, TuiRect, TuiSize,
};
use warpui_core::keymap::Keystroke;

use super::TuiAltScreenElement;

fn model_with_text(text: &str) -> Arc<FairMutex<TerminalModel>> {
    let mut model = TerminalModel::mock(None, None);
    model.set_mode(Mode::SwapScreen {
        save_cursor_and_clear_screen: true,
    });
    for character in text.chars() {
        model.input(character);
    }
    Arc::new(FairMutex::new(model))
}

#[test]
fn paints_alt_screen_grid_and_places_cursor() {
    App::test((), |app| async move {
        app.read(|app| {
            let mut element = TuiAltScreenElement::new(model_with_text("hello"));
            let mut rendered_views = EntityIdMap::default();
            let mut layout_ctx = TuiLayoutContext {
                rendered_views: &mut rendered_views,
            };
            let area = TuiRect::new(0, 0, 10, 3);
            assert_eq!(
                element.layout(
                    TuiConstraint::tight(TuiSize::new(area.width, area.height)),
                    &mut layout_ctx,
                    app,
                ),
                TuiSize::new(10, 3)
            );

            let mut buffer = TuiBuffer::empty(area);
            let mut paint_ctx = TuiPaintContext::new(&mut rendered_views);
            element.render(area, &mut buffer, &mut paint_ctx);

            assert_eq!(buffer.to_lines()[0], "hello     ");
            assert_eq!(element.cursor_position(area, &mut paint_ctx), Some((5, 0)));
        });
    });
}

#[test]
fn key_events_are_encoded_for_the_pty() {
    let element = TuiAltScreenElement::new(model_with_text(""));
    for (keystroke, expected) in [
        ("ctrl-c", vec![0x03]),
        ("enter", vec![b'\r']),
        ("escape", vec![0x1b]),
        ("tab", vec![b'\t']),
        ("shift-tab", b"\x1b[Z".to_vec()),
        ("delete", b"\x1b[3~".to_vec()),
    ] {
        let event = TuiEvent::KeyDown {
            keystroke: Keystroke::parse(keystroke).unwrap(),
            chars: String::new(),
            details: KeyEventDetails::default(),
            is_composing: false,
        };
        assert_eq!(
            element.event_bytes(&event, TuiRect::new(0, 0, 10, 3)),
            Some(expected),
            "wrong PTY bytes for {keystroke}"
        );
    }
}

#[test]
fn wheel_events_use_alternate_scroll_without_mouse_reporting() {
    let element = TuiAltScreenElement::new(model_with_text(""));
    let event = TuiEvent::ScrollWheel {
        position: TuiPoint::new(2, 1),
        delta: (0, 2),
        precise: false,
        modifiers: ModifiersState::default(),
    };

    assert_eq!(
        element.event_bytes(&event, TuiRect::new(0, 0, 10, 3)),
        Some(b"\x1bOA\x1bOA".to_vec())
    );
}

#[test]
fn mouse_reporting_preserves_drag_button_position_and_modifiers() {
    let model = model_with_text("");
    {
        let mut model = model.lock();
        model.set_mode(Mode::SgrMouse);
        model.set_mode(Mode::ReportCellMouseMotion);
    }
    let element = TuiAltScreenElement::new(model);
    let event = TuiEvent::MiddleMouseDragged {
        position: TuiPoint::new(2, 1),
        modifiers: ModifiersState {
            shift: true,
            ctrl: true,
            ..Default::default()
        },
    };

    assert_eq!(
        element.event_bytes(&event, TuiRect::new(0, 0, 10, 3)),
        Some(b"\x1b[<53;3;2M".to_vec())
    );
}
