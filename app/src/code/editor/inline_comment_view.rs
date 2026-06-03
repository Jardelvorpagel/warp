use warpui::elements::{
    ChildView, Container, CornerRadius, CrossAxisAlignment, Flex, ParentElement, Radius,
};
use warpui::units::Pixels;
use warpui::{AppContext, Element, Entity, SingletonEntity, View, ViewContext, ViewHandle};

use crate::appearance::Appearance;
use crate::code::editor::comment_editor::{
    create_readonly_comment_markdown_editor, DEFAULT_COMMENT_MAX_WIDTH,
};
use crate::code::editor::EditorReviewComment;
use crate::notebooks::editor::view::RichTextEditorView;
use crate::ui_components::blended_colors;

/// A per-comment read-only view of a saved code-review comment, hosted inline in the diff editor.
///
/// It owns the full [`EditorReviewComment`] (the editor's slice of the `ReviewCommentBatch` source
/// of truth) plus a read-only markdown body editor. The owning [`CodeEditorView`] keeps a
/// `HashMap<CommentId, ViewHandle<InlineCommentView>>` and reconciles it from
/// `set_comment_locations`: the handle is reused (entity id preserved) and refreshed in place via
/// [`Self::update_source`] when a comment's content changes, so the inline view never thrashes.
pub struct InlineCommentView {
    comment: EditorReviewComment,
    body_editor: ViewHandle<RichTextEditorView>,
}

impl InlineCommentView {
    pub fn new(comment: EditorReviewComment, ctx: &mut ViewContext<Self>) -> Self {
        let body_editor = create_readonly_comment_markdown_editor(
            &comment.comment_content,
            true, /* disable_scrolling */
            Some(Pixels::new(DEFAULT_COMMENT_MAX_WIDTH)),
            ctx,
        );
        Self {
            comment,
            body_editor,
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

    /// The rendered body text of the hosted read-only editor.
    #[cfg(test)]
    pub fn rendered_body(&self, app: &AppContext) -> String {
        self.body_editor
            .as_ref(app)
            .model()
            .as_ref(app)
            .markdown(app)
    }
}

impl Entity for InlineCommentView {
    type Event = ();
}

impl View for InlineCommentView {
    fn ui_name() -> &'static str {
        "InlineCommentView"
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(ctx);
        let theme = appearance.theme();
        let background = blended_colors::neutral_1(theme);

        let column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(ChildView::new(&self.body_editor).finish())
            .finish();

        Container::new(column)
            .with_uniform_padding(8.)
            .with_background_color(background)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
            .finish()
    }
}
