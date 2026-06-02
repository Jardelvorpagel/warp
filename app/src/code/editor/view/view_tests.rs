use std::sync::Arc;

use warp_core::ui::appearance::Appearance;
use warp_editor::content::buffer::InitialBufferState;
use warp_editor::model::CoreEditorModel;
use warp_editor::render::element::VerticalExpansionBehavior;
use warp_editor::render::model::{LineCount, RenderLineLocation};
use warp_util::user_input::UserInput;
use warpui::elements::new_scrollable::ScrollableAppearance;
use warpui::elements::ScrollbarWidth;
use warpui::platform::WindowStyle;
use warpui::{App, TypedActionView, ViewHandle, WindowId};

use super::{CodeEditorRenderOptions, CodeEditorView, CodeEditorViewAction};
use crate::cloud_object::model::persistence::CloudModel;
use crate::code::editor::line::EditorLineLocation;
use crate::editor::InteractionState;
use crate::features::FeatureFlag;
use crate::notebooks::editor::keys::NotebookKeybindings;
use crate::server::server_api::team::MockTeamClient;
use crate::server::server_api::workspace::MockWorkspaceClient;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::vim_registers::VimRegisters;
use crate::workspace::sync_inputs::SyncedInputState;
use crate::workspace::ActiveSession;
use crate::workspaces::user_workspaces::UserWorkspaces;
use crate::AuthStateProvider;

fn initialize_editor(app: &mut App) -> (WindowId, ViewHandle<CodeEditorView>) {
    initialize_settings_for_tests(app);

    // Add all required singleton models for EditorView dependencies
    app.add_singleton_model(|_| Appearance::mock());
    app.add_singleton_model(|_| SyncedInputState::mock());
    app.add_singleton_model(|_| VimRegisters::new());
    app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());

    // Add mocks required by rich text editor (used in CommentEditor)
    app.add_singleton_model(CloudModel::mock);
    app.add_singleton_model(|_| ActiveSession::default());
    app.add_singleton_model(NotebookKeybindings::new);

    // Add UserWorkspaces mock (required by EditorView)
    let team_client_mock = Arc::new(MockTeamClient::new());
    let workspace_client_mock = Arc::new(MockWorkspaceClient::new());
    app.add_singleton_model(|ctx| {
        UserWorkspaces::mock(
            team_client_mock.clone(),
            workspace_client_mock.clone(),
            vec![],
            ctx,
        )
    });

    let (window, editor_view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
        CodeEditorView::new(
            None,
            None,
            CodeEditorRenderOptions::new(VerticalExpansionBehavior::GrowToMaxHeight),
            ctx,
        )
        .with_horizontal_scrollbar_appearance(ScrollableAppearance::new(ScrollbarWidth::Auto, true))
    });

    (window, editor_view)
}

const MULTILINE_CONTENT: &str = "line one\nline two\nline three\nline four\nline five\nline six\n";

fn current_line(line_number: usize) -> EditorLineLocation {
    EditorLineLocation::Current {
        line_number: LineCount::from(line_number),
        line_range: LineCount::from(line_number)..LineCount::from(line_number + 1),
    }
}

/// Pump the executor until both the inner composer editor and the outer code editor have finished
/// laying out, so the inline comment block has converged on its measured height.
async fn settle_layout(app: &mut App, editor: &ViewHandle<CodeEditorView>) {
    for _ in 0..6 {
        let (inner_rs, outer_rs) = app.read(|ctx| {
            let view = editor.as_ref(ctx);
            (
                view.active_comment_editor
                    .as_ref(ctx)
                    .inner_render_state(ctx),
                view.model.as_ref(ctx).render_state().clone(),
            )
        });
        app.read(|ctx| inner_rs.as_ref(ctx).layout_complete()).await;
        app.read(|ctx| outer_rs.as_ref(ctx).layout_complete()).await;
    }
}

async fn await_outer_layout(app: &mut App, editor: &ViewHandle<CodeEditorView>) {
    let outer_rs = app.read(|ctx| editor.as_ref(ctx).model.as_ref(ctx).render_state().clone());
    app.read(|ctx| outer_rs.as_ref(ctx).layout_complete()).await;
}

fn line_offset(app: &App, editor: &ViewHandle<CodeEditorView>, line: usize) -> f32 {
    app.read(|ctx| {
        editor
            .as_ref(ctx)
            .model
            .as_ref(ctx)
            .render_state()
            .as_ref(ctx)
            .vertical_offset_at_render_location(RenderLineLocation::Current(LineCount::from(line)))
            .map(|p| p.as_f32())
            .unwrap_or_default()
    })
}

fn comment_block_height(
    app: &App,
    editor: &ViewHandle<CodeEditorView>,
    line: usize,
) -> Option<f32> {
    app.read(|ctx| {
        editor
            .as_ref(ctx)
            .model
            .as_ref(ctx)
            .render_state()
            .as_ref(ctx)
            .comment_block_position(RenderLineLocation::Current(LineCount::from(line)))
            .map(|position| position.content_height.as_f32())
    })
}

/// VAL-COMPOSER-001/002: with the flag ON, opening the composer inline reserves real vertical
/// space at the clicked line and pushes the line below it down by the composer's height.
#[test]
fn test_inline_composer_pushes_lines_down_when_flag_on() {
    App::test((), |mut app| async move {
        let _inline = FeatureFlag::InlineCodeReview.override_enabled(true);
        let _embedded = FeatureFlag::EmbeddedCodeReviewComments.override_enabled(true);

        let (_window, editor) = initialize_editor(&mut app);
        editor.update(&mut app, |view, ctx| {
            view.reset(InitialBufferState::plain_text(MULTILINE_CONTENT), ctx);
        });
        await_outer_layout(&mut app, &editor).await;

        let baseline_line_3 = line_offset(&app, &editor, 3);
        assert!(
            comment_block_height(&app, &editor, 2).is_none(),
            "no inline composer block should exist before opening"
        );

        editor.update(&mut app, |view, ctx| {
            view.handle_action(
                &CodeEditorViewAction::NewCommentOnLine {
                    line: current_line(2),
                },
                ctx,
            );
        });
        settle_layout(&mut app, &editor).await;

        let block_height = comment_block_height(&app, &editor, 2)
            .expect("an inline composer block should exist at the opened line");
        assert!(
            block_height > 0.0,
            "the inline composer must reserve positive height, got {block_height}"
        );

        let shifted_line_3 = line_offset(&app, &editor, 3);
        let delta = shifted_line_3 - baseline_line_3;
        assert!(
            (delta - block_height).abs() < 1.0,
            "line below should shift down by the composer height: delta={delta}, block_height={block_height}"
        );
    });
}

/// VAL-COMPOSER-011 / VAL-ISOLATION-004 (composer half): with the flag OFF, opening the composer
/// must NOT create an inline comment block, and lines below must not shift (the floating overlay is
/// used instead).
#[test]
fn test_inline_composer_not_inline_when_flag_off() {
    App::test((), |mut app| async move {
        let _inline = FeatureFlag::InlineCodeReview.override_enabled(true);
        let _embedded = FeatureFlag::EmbeddedCodeReviewComments.override_enabled(false);

        let (_window, editor) = initialize_editor(&mut app);
        editor.update(&mut app, |view, ctx| {
            view.reset(InitialBufferState::plain_text(MULTILINE_CONTENT), ctx);
        });
        await_outer_layout(&mut app, &editor).await;

        let baseline_line_3 = line_offset(&app, &editor, 3);

        editor.update(&mut app, |view, ctx| {
            view.handle_action(
                &CodeEditorViewAction::NewCommentOnLine {
                    line: current_line(2),
                },
                ctx,
            );
        });
        settle_layout(&mut app, &editor).await;

        assert!(
            comment_block_height(&app, &editor, 2).is_none(),
            "no inline comment block must exist while the flag is off"
        );
        let line_3 = line_offset(&app, &editor, 3);
        assert!(
            (line_3 - baseline_line_3).abs() < 1.0,
            "line below must not shift while the flag is off: baseline={baseline_line_3}, after={line_3}"
        );
    });
}

/// VAL-COMPOSER-006: cancelling the composer removes the inline block and restores layout.
#[test]
fn test_inline_composer_cancel_restores_layout() {
    App::test((), |mut app| async move {
        let _inline = FeatureFlag::InlineCodeReview.override_enabled(true);
        let _embedded = FeatureFlag::EmbeddedCodeReviewComments.override_enabled(true);

        let (_window, editor) = initialize_editor(&mut app);
        editor.update(&mut app, |view, ctx| {
            view.reset(InitialBufferState::plain_text(MULTILINE_CONTENT), ctx);
        });
        await_outer_layout(&mut app, &editor).await;
        let baseline_line_3 = line_offset(&app, &editor, 3);

        editor.update(&mut app, |view, ctx| {
            view.handle_action(
                &CodeEditorViewAction::NewCommentOnLine {
                    line: current_line(2),
                },
                ctx,
            );
        });
        settle_layout(&mut app, &editor).await;
        assert!(comment_block_height(&app, &editor, 2).is_some());

        // Cancel via the comment editor's close action.
        editor.update(&mut app, |view, ctx| {
            view.active_comment_editor.update(ctx, |composer, ctx| {
                use crate::code::editor::comment_editor::CommentEditorAction;
                composer.handle_action(&CommentEditorAction::CloseEditor, ctx);
            });
        });
        settle_layout(&mut app, &editor).await;

        assert!(
            comment_block_height(&app, &editor, 2).is_none(),
            "cancelling should remove the inline composer block"
        );
        let line_3 = line_offset(&app, &editor, 3);
        assert!(
            (line_3 - baseline_line_3).abs() < 1.0,
            "layout should be restored after cancel: baseline={baseline_line_3}, after={line_3}"
        );
    });
}

#[test]
fn test_interaction_state_prevents_editing() {
    App::test((), |mut app| async move {
        let (_window, editor_view) = initialize_editor(&mut app);

        let text = editor_view.update(&mut app, |view, ctx| {
            view.handle_action(&CodeEditorViewAction::UserTyped(UserInput::new("abc")), ctx);
            view.text(ctx)
        });

        assert_eq!(text.as_str(), "abc");

        // Set to be only selectable
        editor_view.update(&mut app, |view, ctx| {
            view.set_interaction_state(InteractionState::Selectable, ctx);
        });

        let text = editor_view.update(&mut app, |view, ctx| {
            view.handle_action(&CodeEditorViewAction::UserTyped(UserInput::new("def")), ctx);
            view.text(ctx)
        });

        assert_eq!(text.as_str(), "abc");
    });
}
