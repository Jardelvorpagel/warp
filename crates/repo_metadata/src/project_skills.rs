use std::collections::HashSet;
use std::path::Path;

use warp_util::standardized_path::StandardizedPath;

/// Metadata for a project skill file discovered during repository traversal.
///
/// Project skills are intentionally stored outside the canonical file tree so
/// gitignored skill files remain discoverable without becoming visible through
/// generic repository-content APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectSkillFileMetadata {
    pub path: StandardizedPath,
}

impl ProjectSkillFileMetadata {
    pub(crate) fn from_path(path: &Path) -> Self {
        Self {
            path: StandardizedPath::from_local_absolute_unchecked(path),
        }
    }
}

pub(crate) fn replace_project_skill_subtrees(
    project_skill_files: &mut Vec<ProjectSkillFileMetadata>,
    removed_roots: &[StandardizedPath],
    discovered: Vec<ProjectSkillFileMetadata>,
) -> bool {
    let previous = project_skill_files.iter().cloned().collect::<HashSet<_>>();
    let discovered_skill = !discovered.is_empty();
    project_skill_files.retain(|skill| {
        !removed_roots
            .iter()
            .any(|root| skill.path.starts_with(root))
    });
    project_skill_files.extend(discovered);
    project_skill_files.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
    project_skill_files.dedup();
    discovered_skill || previous != project_skill_files.iter().cloned().collect()
}
