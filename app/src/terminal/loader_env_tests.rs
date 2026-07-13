use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

use super::{loader_env_mutations_from, strip_warp_entries, LoaderEnvMutation};

const WARP_DIR: &str = "/opt/warpdotdev/warp-terminal";

/// Build an env lookup closure from a fixed map for deterministic tests.
fn env_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name: &str| map.get(name).map(OsString::from)
}

/// Collect mutations into a map keyed by variable for easy assertions. `None`
/// marks a `Remove`, `Some(value)` a `Set`.
fn mutations_map(
    mutations: Vec<(&'static str, LoaderEnvMutation)>,
) -> HashMap<&'static str, Option<String>> {
    mutations
        .into_iter()
        .map(|(var, mutation)| match mutation {
            LoaderEnvMutation::Set(value) => (var, Some(value)),
            LoaderEnvMutation::Remove => (var, None),
        })
        .collect()
}

#[test]
fn strips_warp_entry_keeps_user_entry() {
    let value = format!("{WARP_DIR}/lib:/home/user/.local/lib");
    let sanitized = strip_warp_entries("LD_LIBRARY_PATH", &value, Some(Path::new(WARP_DIR)));
    assert_eq!(sanitized, "/home/user/.local/lib");
}

#[test]
fn strips_warp_dir_itself() {
    let value = format!("/usr/lib:{WARP_DIR}:/home/user/lib");
    let sanitized = strip_warp_entries("LD_LIBRARY_PATH", &value, Some(Path::new(WARP_DIR)));
    assert_eq!(sanitized, "/usr/lib:/home/user/lib");
}

#[test]
fn leaves_non_warp_entries_unchanged() {
    let value = "/usr/lib:/home/user/.local/lib";
    let sanitized = strip_warp_entries("LD_LIBRARY_PATH", value, Some(Path::new(WARP_DIR)));
    assert_eq!(sanitized, value);
}

#[test]
fn strips_all_when_only_warp_entries() {
    let value = format!("{WARP_DIR}:{WARP_DIR}/lib");
    let sanitized = strip_warp_entries("LD_LIBRARY_PATH", &value, Some(Path::new(WARP_DIR)));
    assert_eq!(sanitized, "");
}

#[test]
fn no_warp_dir_means_no_stripping() {
    let value = format!("{WARP_DIR}/lib:/usr/lib");
    let sanitized = strip_warp_entries("LD_LIBRARY_PATH", &value, None);
    assert_eq!(sanitized, value);
}

#[test]
fn ld_preload_splits_on_whitespace_and_colon() {
    let value = format!("{WARP_DIR}/libwarp.so /home/user/pre.so:{WARP_DIR}/x.so");
    let sanitized = strip_warp_entries("LD_PRELOAD", &value, Some(Path::new(WARP_DIR)));
    assert_eq!(sanitized, "/home/user/pre.so");
}

#[test]
fn mutation_strips_warp_entry() {
    let env = env_from(&[("LD_LIBRARY_PATH", &format!("{WARP_DIR}/lib:/home/user/lib"))]);
    let map = mutations_map(loader_env_mutations_from(env, Some(Path::new(WARP_DIR))));
    assert_eq!(
        map.get("LD_LIBRARY_PATH"),
        Some(&Some("/home/user/lib".to_owned()))
    );
}

#[test]
fn mutation_removes_var_when_only_warp_entry() {
    let env = env_from(&[("LD_LIBRARY_PATH", &format!("{WARP_DIR}/lib"))]);
    let map = mutations_map(loader_env_mutations_from(env, Some(Path::new(WARP_DIR))));
    assert_eq!(map.get("LD_LIBRARY_PATH"), Some(&None));
}

#[test]
fn mutation_absent_when_no_warp_entry() {
    // A value with no Warp entries should produce no mutation at all.
    let env = env_from(&[("LD_LIBRARY_PATH", "/usr/lib:/home/user/lib")]);
    let map = mutations_map(loader_env_mutations_from(env, Some(Path::new(WARP_DIR))));
    assert!(!map.contains_key("LD_LIBRARY_PATH"));
}

#[test]
fn backup_var_restores_original_value_and_is_dropped() {
    let env = env_from(&[
        ("LD_LIBRARY_PATH", &format!("{WARP_DIR}/lib:/home/user/lib")),
        ("WARP_ORIGINAL_LD_LIBRARY_PATH", "/home/user/lib"),
    ]);
    let map = mutations_map(loader_env_mutations_from(env, Some(Path::new(WARP_DIR))));
    // Restored from the backup, not from the strip heuristic.
    assert_eq!(
        map.get("LD_LIBRARY_PATH"),
        Some(&Some("/home/user/lib".to_owned()))
    );
    // The backup var must never leak into children.
    assert_eq!(map.get("WARP_ORIGINAL_LD_LIBRARY_PATH"), Some(&None));
}

#[test]
fn empty_backup_var_removes_target_var() {
    // The user had no LD_LIBRARY_PATH before Warp; the launcher recorded an
    // empty backup, so the variable should be removed entirely.
    let env = env_from(&[
        ("LD_LIBRARY_PATH", &format!("{WARP_DIR}/lib")),
        ("WARP_ORIGINAL_LD_LIBRARY_PATH", ""),
    ]);
    let map = mutations_map(loader_env_mutations_from(env, Some(Path::new(WARP_DIR))));
    assert_eq!(map.get("LD_LIBRARY_PATH"), Some(&None));
    assert_eq!(map.get("WARP_ORIGINAL_LD_LIBRARY_PATH"), Some(&None));
}

#[test]
fn no_loader_vars_means_no_mutations() {
    let env = env_from(&[("PATH", "/usr/bin")]);
    let mutations = loader_env_mutations_from(env, Some(Path::new(WARP_DIR)));
    assert!(mutations.is_empty());
}
