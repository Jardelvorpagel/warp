use super::*;

fn repo_path(path: &str) -> PathBuf {
    std::env::temp_dir()
        .join("repo_metadata_standing_queries_tests")
        .join(path)
}

fn standardized(path: &Path) -> StandardizedPath {
    StandardizedPath::try_from_local(path).unwrap()
}

#[test]
fn records_project_rules_without_project_skills() {
    let definitions = StandingQueryDefinitions::default();
    let mut results = StandingQueryResults::default();
    let skill_file = repo_path(".agents/skills/review/SKILL.md");
    let root_rule = repo_path("WARP.md");
    let nested_rule = repo_path("packages/api/AGENTS.md");

    results.record_path(&skill_file, false, &definitions);
    results.record_path(&root_rule, false, &definitions);
    results.record_path(&nested_rule, false, &definitions);

    assert!(results
        .project_rules()
        .any(|content| content == &StandingQueryContent::file(standardized(&root_rule))));
    assert!(results
        .project_rules()
        .any(|content| content == &StandingQueryContent::file(standardized(&nested_rule))));
    assert!(results
        .project_rules()
        .all(|content| content.path != standardized(&skill_file)));
}

#[test]
fn replacing_rule_subtrees_returns_rule_only_delta() {
    let rule_path = repo_path("packages/api/AGENTS.md");
    let removed_root = repo_path("packages/api");
    let rule = StandingQueryContent::file(standardized(&rule_path));
    let mut results = StandingQueryResults::default();
    results.insert_project_rule(rule.clone());

    let delta = results.replace_subtrees(&[standardized(&removed_root)], Default::default());

    assert_eq!(delta.removed_project_rules, vec![rule]);
    assert!(delta.upserted_project_rules.is_empty());
    assert!(results.project_rules().next().is_none());
}
