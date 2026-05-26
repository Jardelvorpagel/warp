use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Debug, Clone)]
pub struct GitignoreRules {
    root_path: PathBuf,
    entries: HashMap<GitignoreRuleKey, GitignoreRule>,
}

#[derive(Debug, Clone)]
struct GitignoreRule {
    matcher: Gitignore,
    scope: PathBuf,
    metadata: Option<GitignoreFileMetadata>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum GitignoreRuleKey {
    Global,
    File(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GitignoreFileMetadata {
    modified: Option<SystemTime>,
    len: u64,
}

pub struct GitignoreTraversal<'a> {
    rules: &'a mut GitignoreRules,
    active_keys: Vec<GitignoreRuleKey>,
}

impl GitignoreRules {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            entries: HashMap::new(),
        }
    }

    pub fn for_directory(root_path: &Path) -> Self {
        let mut rules = Self::new(root_path);
        rules.refresh_global();
        rules.refresh_gitignore_for_directory(root_path);
        rules
    }

    pub fn for_directory_with_supported_ignores(
        root_path: &Path,
        supported_ignore_files: &[&str],
    ) -> Self {
        let mut rules = Self::for_directory(root_path);
        for supported_ignore_file in supported_ignore_files {
            rules.refresh_ignore_file(
                root_path.join(supported_ignore_file),
                root_path.to_path_buf(),
            );
        }
        rules
    }

    pub fn matcher_count(&self) -> usize {
        self.entries.len()
    }

    pub fn traversal_for_path(&mut self, path: &Path) -> GitignoreTraversal<'_> {
        self.refresh_ancestor_gitignores(path);
        let active_keys = self.applicable_keys_for_path(path);
        GitignoreTraversal {
            rules: self,
            active_keys,
        }
    }

    pub fn is_ignored(&mut self, path: &Path, is_dir: bool, check_ancestors: bool) -> bool {
        self.refresh_ancestor_gitignores(path);
        self.applicable_keys_for_path(path).iter().any(|key| {
            self.entries.get(key).is_some_and(|entry| {
                gitignore_matches_path(&entry.matcher, path, is_dir, check_ancestors)
            })
        })
    }

    fn refresh_global(&mut self) {
        let (gitignore, _) = GitignoreBuilder::new(&self.root_path).build_global();
        if !gitignore.is_empty() {
            self.entries.insert(
                GitignoreRuleKey::Global,
                GitignoreRule {
                    matcher: gitignore,
                    scope: self.root_path.clone(),
                    metadata: None,
                },
            );
        }
    }

    fn refresh_ancestor_gitignores(&mut self, path: &Path) {
        let directory_path = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };

        if !directory_path.starts_with(&self.root_path) {
            return;
        }

        let mut ancestors = directory_path
            .ancestors()
            .take_while(|ancestor| ancestor.starts_with(&self.root_path))
            .collect::<Vec<_>>();
        ancestors.reverse();

        for ancestor in ancestors {
            self.refresh_gitignore_for_directory(ancestor);
        }
    }

    fn refresh_gitignore_for_directory(&mut self, directory_path: &Path) {
        self.refresh_ignore_file(
            directory_path.join(".gitignore"),
            directory_path.to_path_buf(),
        );
    }

    fn refresh_ignore_file(&mut self, ignore_file_path: PathBuf, scope: PathBuf) {
        let key = GitignoreRuleKey::File(ignore_file_path.clone());
        let metadata = gitignore_file_metadata(&ignore_file_path);

        let Some(metadata) = metadata else {
            self.entries.remove(&key);
            return;
        };

        if self
            .entries
            .get(&key)
            .is_some_and(|entry| entry.metadata == Some(metadata))
        {
            return;
        }

        let (matcher, _) = Gitignore::new(&ignore_file_path);
        self.entries.insert(
            key,
            GitignoreRule {
                matcher,
                scope,
                metadata: Some(metadata),
            },
        );
    }

    fn applicable_keys_for_path(&self, path: &Path) -> Vec<GitignoreRuleKey> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| {
                gitignore_applies_to_path(&entry.scope, path).then_some(key.clone())
            })
            .collect()
    }
}

impl GitignoreTraversal<'_> {
    pub fn enter_directory(&mut self, path: &Path) -> usize {
        let active_len = self.active_keys.len();
        self.rules.refresh_gitignore_for_directory(path);

        let key = GitignoreRuleKey::File(path.join(".gitignore"));
        if self.rules.entries.contains_key(&key) && !self.active_keys.contains(&key) {
            self.active_keys.push(key);
        }

        active_len
    }

    pub fn truncate_active(&mut self, active_len: usize) {
        self.active_keys.truncate(active_len);
    }

    pub fn matches(&self, path: &Path, is_dir: bool, check_ancestors: bool) -> bool {
        self.active_keys.iter().any(|key| {
            self.rules.entries.get(key).is_some_and(|entry| {
                gitignore_matches_path(&entry.matcher, path, is_dir, check_ancestors)
            })
        })
    }
}

fn gitignore_file_metadata(path: &Path) -> Option<GitignoreFileMetadata> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(GitignoreFileMetadata {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

fn gitignore_applies_to_path(scope: &Path, path: &Path) -> bool {
    scope == Path::new("") || path.starts_with(scope)
}
pub(crate) fn gitignore_matches_path(
    gitignore: &Gitignore,
    path: &Path,
    is_dir: bool,
    check_ancestors: bool,
) -> bool {
    if let Ok(relative_path) = path.strip_prefix(gitignore.path()) {
        // `matched_path_or_any_parents` panics if the path has a root.
        // If not on windows, we allow paths with a root if the gitignore path is empty (since this denotes a global gitignore).
        if relative_path.has_root() && (cfg!(windows) || gitignore.path() != Path::new("")) {
            return false;
        }

        if check_ancestors {
            gitignore
                .matched_path_or_any_parents(relative_path, is_dir)
                .is_ignore()
        } else {
            gitignore.matched(relative_path, is_dir).is_ignore()
        }
    } else {
        false
    }
}
