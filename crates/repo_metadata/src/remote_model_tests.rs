use warp_util::standardized_path::StandardizedPath;
use warpui_core::App;

use super::*;
use crate::StandingQueryContent;

fn path(path: &str) -> StandardizedPath {
    StandardizedPath::try_new(path).unwrap()
}

#[test]
fn snapshot_and_incremental_update_maintain_remote_standing_results() {
    App::test((), |mut app| async move {
        let model = app.add_model(RemoteRepoMetadataModel::new);
        let host = HostId::new("remote-host".to_string());
        let repo_path = path("/repo");
        let rule = StandingQueryContent::file(path("/repo/WARP.md"));
        let next_rule = StandingQueryContent::file(path("/repo/AGENTS.md"));
        let id = RemoteRepositoryIdentifier::new(host.clone(), repo_path.clone());
        let snapshot = RepoMetadataUpdate {
            repo_path: repo_path.clone(),
            remove_entries: Vec::new(),
            update_entries: Vec::new(),
            standing_results_delta: StandingQueryResultsDelta {
                upserted_project_rules: vec![rule.clone()],
                ..Default::default()
            },
        };
        model.update(&mut app, |model, ctx| {
            model.insert_from_snapshot(host.clone(), &snapshot, ctx);
        });
        model.read(&app, |model, _ctx| {
            assert!(model
                .standing_query_results(&id)
                .unwrap()
                .project_rules()
                .any(|content| content == &rule));
        });

        let incremental = RepoMetadataUpdate {
            repo_path,
            remove_entries: Vec::new(),
            update_entries: Vec::new(),
            standing_results_delta: StandingQueryResultsDelta {
                removed_project_rules: vec![rule],
                upserted_project_rules: vec![next_rule.clone()],
                ..Default::default()
            },
        };
        model.update(&mut app, |model, ctx| {
            model.apply_incremental_update(&host, &incremental, ctx);
        });

        model.read(&app, |model, _ctx| {
            let results = model.standing_query_results(&id).unwrap();
            assert_eq!(
                results.project_rules().collect::<Vec<_>>(),
                vec![&next_rule]
            );
        });
    });
}
