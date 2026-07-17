use warp::tui_export::{
    export_conversation_markdown, PtyIntent, PtyIntentEvent, SizeInfo, SizeUpdate,
};
use warpui_core::elements::tui::{TuiBufferExt, TuiRect, TuiText};
use warpui_core::keymap::{Context, Keystroke, Trigger};
use warpui_core::presenter::tui::TuiPresenter;
use warpui_core::App;

use super::{
    contextual_keyboard_hint, export_file_success_message, raw_prompt_if_not_blank,
    TuiTerminalSessionEvent, IDLE_KEYBOARD_HINT, WARPING_KEYBOARD_HINT,
};
use crate::keybindings::{
    CONTEXTUAL_PLAN_TOGGLE_BINDING_NAME, KEYBOARD_ENHANCEMENT_AVAILABLE_FLAG,
    PLAN_TOGGLE_AVAILABLE_FLAG, PLAN_TOGGLE_BINDING_NAME,
};

/// Checks that `contextual_keyboard_hint` selects the correct hint text based
/// on whether the active conversation has an in-flight agent response. These
/// are the hint strings rendered in the footer's left slot.
#[test]
fn contextual_hint_idle_when_not_in_progress() {
    assert_eq!(contextual_keyboard_hint(false), IDLE_KEYBOARD_HINT);
    assert_eq!(
        contextual_keyboard_hint(false),
        "↑ to edit  ← for conversations"
    );
}

#[test]
fn contextual_hint_warping_when_in_progress() {
    assert_eq!(contextual_keyboard_hint(true), WARPING_KEYBOARD_HINT);
    assert_eq!(contextual_keyboard_hint(true), "/ for commands");
}

/// Render-to-lines: verifies the idle hint (post-response state) produces the
/// expected glyph row when rendered as a footer left-slot text element.
#[test]
fn footer_idle_keyboard_hint_renders_to_expected_row() {
    App::test((), |mut app| async move {
        app.update(|ctx| {
            let hint = contextual_keyboard_hint(false);
            let frame = TuiPresenter::new().present_element(
                TuiText::new(hint).truncate().finish(),
                TuiRect::new(0, 0, 40, 1),
                ctx,
            );
            let line = frame
                .buffer
                .to_lines()
                .into_iter()
                .next()
                .unwrap_or_default();
            assert!(
                line.starts_with("↑ to edit  ← for conversations"),
                "idle footer hint row should start with expected keyboard hints, got: {line:?}"
            );
        });
    });
}

/// Render-to-lines: verifies the warping hint (in-progress state) produces the
/// expected glyph row when rendered as a footer left-slot text element.
#[test]
fn footer_warping_keyboard_hint_renders_to_expected_row() {
    App::test((), |mut app| async move {
        app.update(|ctx| {
            let hint = contextual_keyboard_hint(true);
            let frame = TuiPresenter::new().present_element(
                TuiText::new(hint).truncate().finish(),
                TuiRect::new(0, 0, 20, 1),
                ctx,
            );
            let line = frame
                .buffer
                .to_lines()
                .into_iter()
                .next()
                .unwrap_or_default();
            assert!(
                line.starts_with("/ for commands"),
                "warping footer hint row should start with '/ for commands', got: {line:?}"
            );
        });
    });
}
#[test]
fn interrupt_event_projects_to_high_level_pty_intent() {
    let event = TuiTerminalSessionEvent::InterruptPty;
    assert!(matches!(event.pty_intent(), Some(PtyIntent::Interrupt)));
}

#[test]
fn user_input_event_projects_to_raw_user_bytes() {
    let event = TuiTerminalSessionEvent::WriteUserInput(b"hello\r".to_vec().into());
    let Some(PtyIntent::WriteBytes(bytes)) = event.pty_intent() else {
        panic!("user input event should map to raw PTY bytes");
    };
    assert_eq!(&*bytes, b"hello\r");
}
#[test]
fn plan_toggle_uses_contextual_ctrl_p_and_ctrl_shift_p() {
    App::test((), |mut app| async move {
        app.update(crate::keybindings::init);
        app.read(|ctx| {
            let toggle = ctx
                .get_binding_by_name(PLAN_TOGGLE_BINDING_NAME)
                .expect("primary plan toggle binding");
            assert_eq!(
                *toggle.trigger,
                Trigger::Keystrokes(vec![Keystroke::parse("ctrl-shift-P").unwrap()])
            );

            let fallback = ctx
                .editable_bindings()
                .find(|binding| binding.name == CONTEXTUAL_PLAN_TOGGLE_BINDING_NAME)
                .expect("contextual plan toggle binding");
            let ctrl_p = Trigger::Keystrokes(vec![Keystroke::parse("ctrl-p").unwrap()]);
            assert_eq!(*fallback.trigger, ctrl_p);

            let mut input_without_plan = Context::default();
            input_without_plan.set.insert("TuiInputView");
            let mut input_with_plan = input_without_plan.clone();
            input_with_plan.set.insert(PLAN_TOGGLE_AVAILABLE_FLAG);
            let mut enhanced_input_with_plan = input_with_plan.clone();
            enhanced_input_with_plan
                .set
                .insert(KEYBOARD_ENHANCEMENT_AVAILABLE_FLAG);
            assert!(!fallback.in_context(&input_without_plan));
            assert!(fallback.in_context(&input_with_plan));
            assert!(!fallback.in_context(&enhanced_input_with_plan));

            let ctrl_p_move_up = ctx
                .editable_bindings()
                .find(|binding| binding.name == "tui:input:move_up" && *binding.trigger == ctrl_p)
                .expect("Ctrl+P move-up fallback");
            assert!(ctrl_p_move_up.in_context(&input_without_plan));
            assert!(!ctrl_p_move_up.in_context(&input_with_plan));
            assert!(ctrl_p_move_up.in_context(&enhanced_input_with_plan));
        });
    });
}

#[test]
fn non_command_prompt_preserves_leading_whitespace() {
    assert_eq!(raw_prompt_if_not_blank("  /compact"), Some("  /compact"));
}

#[test]
fn whitespace_only_prompt_is_ignored() {
    assert_eq!(raw_prompt_if_not_blank(" \t\n"), None);
}

#[test]
fn file_export_success_message_includes_destination_path() {
    let directory = tempfile::tempdir().expect("temp directory");
    let export = export_conversation_markdown(
        Some(directory.path().to_str().expect("UTF-8 temp path")),
        Some("conversation.md"),
        None,
        "# Conversation",
    )
    .expect("conversation export");

    assert_eq!(
        export_file_success_message(&export),
        format!("Conversation exported to {}", export.path().display())
    );
}

#[test]
fn resize_event_maps_to_pty_resize_intent() {
    let last_size = SizeInfo::new_without_font_metrics(24, 120);
    let size_update = SizeUpdate::from_cell_dimensions(last_size, 8, 42);
    let event = TuiTerminalSessionEvent::Resize(size_update);

    let Some(PtyIntent::Resize(actual_update)) = event.pty_intent() else {
        panic!("resize event should map to a PTY resize intent");
    };
    assert_eq!(actual_update.new_size().rows(), 8);
    assert_eq!(actual_update.new_size().columns(), 42);
}
