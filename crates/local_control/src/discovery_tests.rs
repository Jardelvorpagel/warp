use std::fs;
use std::path::Path;

use super::*;

#[test]
fn registered_instance_round_trips_discovery_record() {
    let dir = tempfile::tempdir().expect("temp dir");
    let record = InstanceRecord::for_current_process(
        ControlEndpoint::localhost(4000),
        "local",
        "dev.warp.WarpLocal",
        Some("test".to_owned()),
        crate::protocol::ActionKind::implemented_metadata(),
        true,
    );
    let _registered = RegisteredInstance::register_in_dir_for_test(record.clone(), dir.path())
        .expect("registered");
    let records = list_instances_from_dir(dir.path());
    assert_eq!(records, vec![record]);
}

#[test]
fn serialized_discovery_record_does_not_contain_raw_credential_material() {
    let raw_secret = "raw-secret-token-material";
    let record = InstanceRecord::for_current_process(
        ControlEndpoint::localhost(4000),
        "local",
        "dev.warp.WarpLocal",
        Some("test".to_owned()),
        crate::protocol::ActionKind::implemented_metadata(),
        true,
    );
    let serialized = serde_json::to_string_pretty(&record).expect("serialize");
    assert!(!serialized.contains(raw_secret));
    assert!(!serialized.contains("auth_token"));
    assert!(!serialized.contains("bearer_token"));
}

#[test]
fn disabled_outside_warp_record_does_not_publish_actionable_authority() {
    let record = InstanceRecord::for_current_process(
        ControlEndpoint::localhost(4000),
        "local",
        "dev.warp.WarpLocal",
        Some("test".to_owned()),
        crate::protocol::ActionKind::implemented_metadata(),
        false,
    );
    let serialized = serde_json::to_value(&record).expect("serialize");

    assert_eq!(serialized["outside_warp_control_enabled"], false);
    assert!(serialized.get("endpoint").is_none());
    assert!(serialized.get("credential_broker").is_none());
    assert_eq!(record.actions, Vec::new());
    assert_eq!(
        record.endpoint().expect_err("endpoint is unavailable").code,
        ErrorCode::LocalControlDisabled
    );
    assert_eq!(
        record
            .credential_broker()
            .expect_err("broker is unavailable")
            .code,
        ErrorCode::LocalControlDisabled
    );
}

impl RegisteredInstance {
    fn register_in_dir_for_test(record: InstanceRecord, dir: &Path) -> Result<Self, ControlError> {
        fs::create_dir_all(dir).expect("create dir");
        let path = record_path(dir, &record.instance_id);
        let bytes = serde_json::to_vec_pretty(&record).expect("serialize");
        fs::write(&path, bytes).expect("write");
        Ok(Self { record, path })
    }
}
