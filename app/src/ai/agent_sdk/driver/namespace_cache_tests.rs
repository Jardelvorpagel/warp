use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use build_cache::{
    CacheInvocationReport, CacheInvocationScope, CacheModeReport, CacheSetupOutcome,
};
use warp_isolation_platform::IsolationPlatformType;

use super::{build_cache_root, build_export_command, into_event_report};
use crate::terminal::shell::ShellType;

fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn cache_root_requires_namespace_and_nonempty_absolute_path() {
    assert_eq!(
        build_cache_root(
            Some(IsolationPlatformType::Namespace),
            Some(OsString::from("/cache/build"))
        ),
        Some(PathBuf::from("/cache/build"))
    );
    assert_eq!(
        build_cache_root(
            Some(IsolationPlatformType::Docker),
            Some(OsString::from("/cache/build"))
        ),
        None
    );
    assert_eq!(
        build_cache_root(Some(IsolationPlatformType::Namespace), None),
        None
    );
    assert_eq!(
        build_cache_root(
            Some(IsolationPlatformType::Namespace),
            Some(OsString::new())
        ),
        None
    );
    assert_eq!(
        build_cache_root(
            Some(IsolationPlatformType::Namespace),
            Some(OsString::from("cache/build"))
        ),
        None
    );
}

#[test]
fn export_command_is_single_safe_shell_command_and_rejects_invalid_names() {
    let command = build_export_command(
        &map(&[("ALPHA", "one"), ("CACHE_VALUE", "value with ' quote")]),
        ShellType::Bash,
    )
    .unwrap();
    assert_eq!(
        command,
        "export ALPHA='one'\nexport CACHE_VALUE='value with '\"'\"' quote'"
    );
    assert!(build_export_command(&map(&[("INVALID-NAME", "value")]), ShellType::Bash).is_err());
}

#[test]
fn event_report_contains_only_opaque_repo_key_and_modes() {
    let outcome = CacheSetupOutcome {
        setup_is_error: false,
        invocations: vec![CacheInvocationReport {
            scope: CacheInvocationScope::Repository {
                repo_key: "a".repeat(64),
            },
            is_error: false,
            modes: BTreeMap::from([(
                "go".to_owned(),
                CacheModeReport {
                    cache_hits: 1,
                    cache_misses: 0,
                },
            )]),
        }],
        environment: BTreeMap::new(),
    };

    let debug = format!("{:?}", into_event_report(outcome));

    assert!(debug.contains("repo_key"));
    assert!(debug.contains(&"a".repeat(64)));
    assert!(!debug.contains("workspace"));
    assert!(!debug.contains("cache/build"));
}
