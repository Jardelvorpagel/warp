use std::cell::RefCell;

use chrono::Local;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::theme::Fill;
use warp_editor::render::model::RenderState;
use warpui::elements::{
    Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex,
    MainAxisAlignment, MainAxisSize, ParentElement, Radius, Shrinkable, Text,
};
use warpui::text_layout::ClipConfig;
use warpui::units::Pixels;
use warpui::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use crate::appearance::Appearance;
use crate::code::editor::EditorReviewComment;
use crate::code::editor::comment_editor::{
    DEFAULT_COMMENT_MAX_WIDTH, create_readonly_comment_markdown_editor,
};
use crate::code::editor::line::EditorLineLocation;
use crate::code_review::comments::{CommentId, CommentOrigin};
use crate::notebooks::editor::view::RichTextEditorView;
use crate::ui_components::blended_colors;
use crate::ui_components::icons::Icon;
use crate::util::time_format::human_readable_approx_duration;
use crate::view_components::action_button::{
    ActionButton, ButtonSize, DangerNakedTheme, NakedTheme,
};

/// Fixed vertical chrome around the inner read-only body editor: the editor area's top/bottom
/// padding (8 + 4), the footer's vertical padding and top border (8 + 1), the footer button row
/// (`ButtonSize::Small` is 24px tall), and the outer container's top/bottom border (2). Slightly
/// generous so the reserved inline block is never shorter than the painted card.
const SAVED_CARD_CHROME_HEIGHT: f32 = 48.0;

#[derive(Debug)]
pub enum InlineCommentViewAction {
    /// The card's edit affordance was activated; open the inline composer for this comment.
    Edit,
    /// The card's remove affordance was activated; delete this comment.
    Remove,
}

#[derive(Debug)]
pub enum InlineCommentViewEvent {
    /// The user asked to edit this saved comment inline (via the card's edit affordance). The
    /// hosting [`CodeEditorView`] reopens it as the prefilled inline composer.
    RequestEdit {
        id: CommentId,
        line: EditorLineLocation,
        comment_text: String,
        origin: CommentOrigin,
    },
    /// The user asked to remove this saved comment inline.
    RequestRemove { id: CommentId },
}

/// A per-comment read-only view of a saved code-review comment, hosted inline in the diff editor.
///
/// It owns the full [`EditorReviewComment`] (the editor's slice of the `ReviewCommentBatch` source
/// of truth) plus a read-only markdown body editor. The owning [`CodeEditorView`] keeps a
/// `HashMap<CommentId, ViewHandle<InlineCommentView>>` and reconciles it from
/// `set_comment_locations`: the handle is reused (entity id preserved) and refreshed in place via
/// [`Self::update_source`] when a comment's content changes, so the inline view never thrashes.
///
/// Per the locked design decision the card shows the comment body plus a composer-style footer with
/// lightweight metadata (relative time + an imported-from-GitHub indicator) and edit/remove
/// affordances — but NOT the redundant embedded diff snippet that the bottom-panel
/// `CommentViewCard` renders (the comment already sits on its own diff line inline).
pub struct InlineCommentView {
    comment: EditorReviewComment,
    body_editor: ViewHandle<RichTextEditorView>,
    edit_button: ViewHandle<ActionButton>,
    remove_button: ViewHandle<ActionButton>,
    laid_out_size: RefCell<Option<Vector2F>>,
}

impl InlineCommentView {
    pub fn new(comment: EditorReviewComment, ctx: &mut ViewContext<Self>) -> Self {
        let body_editor = create_readonly_comment_markdown_editor(
            &comment.comment_content,
            true, /* disable_scrolling */
            Some(Pixels::new(DEFAULT_COMMENT_MAX_WIDTH)),
            ctx,
        );
        let edit_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Edit", NakedTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(InlineCommentViewAction::Edit))
        });
        let remove_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Remove", DangerNakedTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(InlineCommentViewAction::Remove))
        });
        Self {
            comment,
            body_editor,
            edit_button,
            remove_button,
            laid_out_size: RefCell::new(None),
        }
    }

    /// Refresh this view's data in place, resetting the body editor only when the content changed.
    /// Reusing the same handle keeps the inline block stable across batch updates.
    pub fn update_source(&mut self, comment: EditorReviewComment, ctx: &mut ViewContext<Self>) {
        if comment.comment_content != self.comment.comment_content {
            self.body_editor.update(ctx, |editor, ctx| {
                editor.model().update(ctx, |model, ctx| {
                    model.reset_with_markdown(&comment.comment_content, ctx);
                });
            });
        }
        self.comment = comment;
        ctx.notify();
    }

    pub fn line(&self) -> &EditorLineLocation {
        &self.comment.line
    }

    /// The render state backing the inner read-only body editor. Observing it lets the host
    /// re-measure the card's reserved inline height when its laid-out height changes (for example
    /// after a width change re-wraps the body).
    pub fn inner_render_state(&self, app: &AppContext) -> ModelHandle<RenderState> {
        self.body_editor
            .as_ref(app)
            .model()
            .as_ref(app)
            .render_state()
            .clone()
    }

    /// The height, in pixels, this card needs to render inline at its line: the body editor's
    /// laid-out content height plus fixed chrome. Saved cards are not height-capped (a tall comment
    /// reserves its full height and scrolls into view with the surrounding code).
    pub fn inline_height(&self, app: &AppContext) -> Pixels {
        let content_height = self.inner_render_state(app).as_ref(app).height().as_f32();
        Pixels::new(content_height + SAVED_CARD_CHROME_HEIGHT)
    }

    pub fn set_laid_out_size(&self, value: Vector2F) {
        self.laid_out_size.replace(Some(value));
    }

    #[allow(unused)]
    pub fn get_laid_out_size(&self) -> Option<Vector2F> {
        self.laid_out_size.borrow().as_ref().cloned()
    }

    /// The rendered body text of the hosted read-only editor.
    #[cfg(any(test, feature = "integration_tests"))]
    pub fn rendered_body(&self, app: &AppContext) -> String {
        self.body_editor
            .as_ref(app)
            .model()
            .as_ref(app)
            .markdown(app)
    }

    /// Override the body editor's soft-wrap max width (test-only), forcing the card's body to
    /// re-wrap. The host re-measures the card's reserved height when the body re-lays out.
    #[cfg(feature = "integration_tests")]
    pub fn set_body_wrap_width_for_test(&mut self, max_width: Pixels, ctx: &mut ViewContext<Self>) {
        self.body_editor.update(ctx, |editor, ctx| {
            editor.set_max_width_for_test(Some(max_width), ctx);
        });
        ctx.notify();
    }

    /// Whether this saved card embeds a diff snippet. The inline card renders only the comment body
    /// and lightweight metadata — never the redundant diff snippet the bottom-panel card shows — so
    /// this is structurally `false`. The getter exists so a regression that started embedding a
    /// snippet inline would surface in the integration tests.
    #[cfg(feature = "integration_tests")]
    pub fn embeds_diff_snippet_for_test(&self) -> bool {
        false
    }

    fn render_metadata(&self, appearance: &Appearance, background: ColorU) -> Box<dyn Element> {
        let theme = appearance.theme();
        let sub_text_color = theme.sub_text_color(Fill::Solid(background)).into_solid();

        let mut leading = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.);

        if self.comment.origin.is_imported_from_github() {
            leading = leading.with_child(
                ConstrainedBox::new(
                    Icon::Github
                        .to_warpui_icon(Fill::Solid(sub_text_color))
                        .finish(),
                )
                .with_width(14.)
                .with_height(14.)
                .finish(),
            );
        }

        let relative_time = human_readable_approx_duration(
            Local::now() - self.comment.last_update_time,
            true, /* sentence_case */
        );
        leading = leading.with_child(
            Text::new(
                relative_time,
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .soft_wrap(false)
            .with_clip(ClipConfig::end())
            .with_color(sub_text_color)
            .finish(),
        );
        leading.finish()
    }

    fn render_action_buttons(&self) -> Box<dyn Element> {
        Flex::row()
            .with_spacing(4.)
            .with_children([
                ChildView::new(&self.edit_button).finish(),
                ChildView::new(&self.remove_button).finish(),
            ])
            .with_main_axis_alignment(MainAxisAlignment::End)
            .finish()
    }

    fn render_footer_row(&self, appearance: &Appearance, background: ColorU) -> Box<dyn Element> {
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Shrinkable::new(1., self.render_metadata(appearance, background)).finish())
            .with_child(self.render_action_buttons())
            .finish()
    }
}

impl Entity for InlineCommentView {
    type Event = InlineCommentViewEvent;
}

impl TypedActionView for InlineCommentView {
    type Action = InlineCommentViewAction;

    fn handle_action(&mut self, action: &InlineCommentViewAction, ctx: &mut ViewContext<Self>) {
        match action {
            InlineCommentViewAction::Edit => {
                ctx.emit(InlineCommentViewEvent::RequestEdit {
                    id: self.comment.id,
                    line: self.comment.line.clone(),
                    comment_text: self.comment.comment_content.clone(),
                    origin: self.comment.origin.clone(),
                });
            }
            InlineCommentViewAction::Remove => {
                ctx.emit(InlineCommentViewEvent::RequestRemove {
                    id: self.comment.id,
                });
            }
        }
    }
}

impl View for InlineCommentView {
    fn ui_name() -> &'static str {
        "InlineCommentView"
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(ctx);
        let theme = appearance.theme();
        let background = blended_colors::neutral_2(theme);
        let border_color = blended_colors::neutral_4(theme);
        let footer_row = self.render_footer_row(appearance, background);

        let column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(ChildView::new(&self.body_editor).finish())
                    .with_padding_bottom(4.)
                    .with_padding_top(8.)
                    .with_horizontal_padding(12.)
                    .finish(),
            )
            .with_child(
                Container::new(footer_row)
                    .with_vertical_padding(4.)
                    .with_horizontal_padding(12.)
                    .with_border(Border::top(1.).with_border_fill(border_color))
                    .finish(),
            )
            .finish();
        ConstrainedBox::new(
            Container::new(column)
                .with_background_color(background)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
                .with_border(Border::all(1.).with_border_fill(border_color))
                .finish(),
        )
        .with_max_width(DEFAULT_COMMENT_MAX_WIDTH)
        .finish()
    }
}
