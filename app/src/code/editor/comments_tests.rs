use chrono::Local;
use warp_editor::render::model::LineCount;

use super::EditorReviewComment;
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code::editor::line::EditorLineLocation;
use crate::code_review::comments::{
    AttachedReviewComment, AttachedReviewCommentTarget, CommentId, CommentOrigin, LineDiffContent,
};

fn attached_comment(
    target: AttachedReviewCommentTarget,
    origin: CommentOrigin,
) -> AttachedReviewComment {
    AttachedReviewComment {
        id: CommentId::new(),
        content: "body".to_string(),
        target,
        last_update_time: Local::now(),
        base: None,
        head: None,
        outdated: false,
        origin,
    }
}

fn line_target() -> AttachedReviewCommentTarget {
    let line = LineCount::from(3usize);
    AttachedReviewCommentTarget::Line {
        absolute_file_path: LocalOrRemotePath::Local("foo.rs".into()),
        line: EditorLineLocation::Current {
            line_number: line,
            line_range: line..LineCount::from(4usize),
        },
        content: LineDiffContent::default(),
    }
}

#[test]
fn try_from_line_comment_preserves_origin_and_content() {
    let comment = attached_comment(line_target(), CommentOrigin::Native);
    let id = comment.id;

    let editor_comment =
        EditorReviewComment::try_from(comment).expect("line comments should convert");

    assert_eq!(editor_comment.id, id);
    assert_eq!(editor_comment.comment_content, "body");
    assert_eq!(editor_comment.origin, CommentOrigin::Native);
}

#[test]
fn try_from_file_comment_returns_err() {
    let comment = attached_comment(
        AttachedReviewCommentTarget::File {
            absolute_file_path: LocalOrRemotePath::Local("foo.rs".into()),
        },
        CommentOrigin::Native,
    );

    assert!(EditorReviewComment::try_from(comment).is_err());
}

#[test]
fn try_from_general_comment_returns_err() {
    let comment = attached_comment(AttachedReviewCommentTarget::General, CommentOrigin::Native);

    assert!(EditorReviewComment::try_from(comment).is_err());
}
