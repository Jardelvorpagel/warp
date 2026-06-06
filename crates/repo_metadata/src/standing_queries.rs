//! Standing repository queries maintained alongside the canonical file tree.
//!
//! These results contain project-derived context paths that must remain available
//! even when the visible file tree is intentionally lazy or shallow.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use warp_util::standardized_path::StandardizedPath;

/// Repository-scoped standing query configuration.
#[derive(Debug, Clone)]
pub struct StandingQueryDefinitions {
    project_skill_provider_paths: Vec<PathBuf>,
    project_rule_file_names: Vec<String>,
}

impl Default for StandingQueryDefinitions {
    fn default() -> Self {
        Self {
            project_skill_provider_paths: Vec::new(),
            project_rule_file_names: vec!["WARP.md".to_string(), "AGENTS.md".to_string()],
        }
    }
}

impl StandingQueryDefinitions {
    pub fn set_project_skill_provider_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.project_skill_provider_paths = paths.into_iter().collect();
    }

    pub fn project_skill_provider_paths(&self) -> &[PathBuf] {
        &self.project_skill_provider_paths
    }

    pub(crate) fn is_project_skill_provider_directory(&self, path: &Path) -> bool {
        self.project_skill_provider_paths
            .iter()
            .any(|provider_path| path.ends_with(provider_path))
    }
    pub(crate) fn is_project_skill_path_interest(&self, path: &Path) -> bool {
        let path_components = path.components().collect::<Vec<_>>();
        self.project_skill_provider_paths
            .iter()
            .any(|provider_path| {
                let provider_components = provider_path.components().collect::<Vec<_>>();
                !provider_components.is_empty()
                    && (path_components
                        .windows(provider_components.len())
                        .any(|window| window == provider_components)
                        || (1..provider_components.len()).any(|prefix_len| {
                            path_components.len() >= prefix_len
                                && path_components[path_components.len() - prefix_len..]
                                    == provider_components[..prefix_len]
                        }))
            })
    }

    pub(crate) fn is_direct_project_skill_provider_child(&self, path: &Path) -> bool {
        path.parent()
            .is_some_and(|parent| self.is_project_skill_provider_directory(parent))
    }

    pub(crate) fn is_project_skill_file(&self, path: &Path) -> bool {
        path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
            && path
                .parent()
                .and_then(Path::parent)
                .is_some_and(|skills_root| self.is_project_skill_provider_directory(skills_root))
    }

    fn is_project_rule_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|file_name| {
                self.project_rule_file_names
                    .iter()
                    .any(|rule_name| file_name.eq_ignore_ascii_case(rule_name))
            })
    }
}

/// A path retained by a standing query.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StandingQueryContent {
    pub path: StandardizedPath,
    pub is_directory: bool,
}

impl StandingQueryContent {
    pub fn file(path: StandardizedPath) -> Self {
        Self {
            path,
            is_directory: false,
        }
    }

    pub fn directory(path: StandardizedPath) -> Self {
        Self {
            path,
            is_directory: true,
        }
    }
}

/// Current paths matching each standing repository query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StandingQueryResults {
    project_rules: HashSet<StandingQueryContent>,
}

impl StandingQueryResults {
    pub fn project_rules(&self) -> impl Iterator<Item = &StandingQueryContent> {
        self.project_rules.iter()
    }

    /// Records a path encountered while traversing the repository.
    pub(crate) fn record_path(
        &mut self,
        path: &Path,
        is_directory: bool,
        definitions: &StandingQueryDefinitions,
    ) {
        let standardized = StandardizedPath::from_local_absolute_unchecked(path);
        if !is_directory && definitions.is_project_rule_file(path) {
            self.project_rules
                .insert(StandingQueryContent::file(standardized));
        }
    }

    pub fn insert_project_rule(&mut self, content: StandingQueryContent) {
        self.project_rules.insert(content);
    }

    pub fn apply_delta(&mut self, delta: &StandingQueryResultsDelta) {
        for removed in &delta.removed_project_rules {
            self.project_rules.remove(removed);
        }
        self.project_rules
            .extend(delta.upserted_project_rules.iter().cloned());
    }

    /// Replaces results beneath changed roots and returns the observable delta.
    ///
    /// Upserts are emitted even when a matching path already exists so consumers
    /// reread modified skill and rules file contents.
    pub fn replace_subtrees(
        &mut self,
        removed_roots: &[StandardizedPath],
        discovered: StandingQueryResults,
    ) -> StandingQueryResultsDelta {
        let mut delta = StandingQueryResultsDelta::default();
        for root in removed_roots {
            let removed_rules = self
                .project_rules
                .iter()
                .filter(|content| content.path.starts_with(root))
                .cloned()
                .collect::<Vec<_>>();
            delta.removed_project_rules.extend(removed_rules);
        }
        delta
            .upserted_project_rules
            .extend(discovered.project_rules);
        self.apply_delta(&delta);
        delta
    }

    pub fn as_snapshot_delta(&self) -> StandingQueryResultsDelta {
        StandingQueryResultsDelta {
            upserted_project_rules: self.project_rules.iter().cloned().collect(),
            removed_project_rules: Vec::new(),
        }
    }
}

/// Changes to standing query results for one repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StandingQueryResultsDelta {
    pub upserted_project_rules: Vec<StandingQueryContent>,
    pub removed_project_rules: Vec<StandingQueryContent>,
}

impl StandingQueryResultsDelta {
    pub fn is_empty(&self) -> bool {
        self.upserted_project_rules.is_empty() && self.removed_project_rules.is_empty()
    }

    pub fn project_rules_changed(&self) -> bool {
        !self.upserted_project_rules.is_empty() || !self.removed_project_rules.is_empty()
    }
}

#[cfg(test)]
#[path = "standing_queries_tests.rs"]
mod tests;
