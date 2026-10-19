use serde_json::Value;
use sha2::{Digest, Sha256};
fn reference() -> Value {
    serde_json::from_str(include_str!("../../../data/preview-backend-fixtures.json")).unwrap()
}
#[test]
fn original_backend_observations_distinguish_headless_start_from_null_and_rendering() {
    let r = reference();
    assert_eq!(r["lab_profile_certified"], false);
    for version in ["0.15.0", "0.15.1"] {
        let rows = r["observations"][version].as_array().unwrap();
        let find = |name: &str| rows.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(find("headless-only")["accepted"], false);
        assert_eq!(find("headless-failed-drm-filtered")["accepted"], false);
        assert_eq!(find("null-only")["accepted"], true);
        assert_eq!(find("null-only")["allocator"], false);
        assert_eq!(find("headless-before-null")["accepted"], false);
        assert_eq!(find("null-before-headless")["accepted"], true);
        assert_eq!(find("reopen-refused")["allocations"], 0);
        assert_eq!(
            find("first-allocator-failure-no-second-try")["allocations"],
            1
        );
        assert_eq!(
            rows.last().unwrap()["headless_fds"],
            serde_json::json!([-1, -1])
        );
    }
}
#[test]
fn exact_sources_and_both_compositor_request_policies_remain_pinned() {
    let r = reference();
    let manifest = include_bytes!("../../../data/preview-backend-sources.json");
    assert_eq!(
        r["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(manifest))
    );
    for version in ["0.56.2", "audited-main"] {
        assert_eq!(
            r["hyprland_requested_backends"][version],
            serde_json::json!([[2, 0], [1, 1], [0, 2]])
        );
    }
    let sources: Value = serde_json::from_slice(manifest).unwrap();
    assert!(sources["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["url"]
            .as_str()
            .unwrap()
            .contains(row["commit"].as_str().unwrap())));
}
