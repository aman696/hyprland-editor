use hyprland_editor_core::{
    catalogue,
    device::{self, Kind, Spec},
    lua,
    workspace::Workspace,
    Draft, Value, AUDITED_MAIN, BASELINE,
};
const LEGACY: &str = "f555d3d357acca4e74b4472f9a7fc441c6f347b8a2b47c8b6d466dbde45966b7";
fn additions() -> Spec {
    let mut spec = Spec::new("pen", "Example Pen");
    spec.kind = Kind::Tablet;
    for (key, value) in [
        ("eraser_button_mode", Value::Int(1)),
        ("eraser_button_override", Value::Int(331)),
        ("pressure_range_min", Value::Float(0.125)),
        ("pressure_range_max", Value::Float(0.875)),
    ] {
        spec.fields.insert(key.into(), value);
    }
    spec
}
#[test]
fn tablet_tool_additions_round_trip_source_and_private_drafts_without_inserting_unset_fields() {
    for version in [BASELINE, AUDITED_MAIN] {
        let spec = additions();
        spec.validate(version).unwrap();
        for key in device::ADDED_KEYS {
            assert_eq!(device::relevance(key), &[Kind::Tablet]);
        }
        let source = spec.export_lua(version).unwrap();
        let report = lua::parse(&source, version, &catalogue()).unwrap();
        assert!(!report.opaque_effects, "{:?}", report.diagnostics);
        assert_eq!(report.devices[0].fields, spec.fields);
        let saved = Workspace::new(Draft::new(version)).with_devices(vec![spec]);
        let bytes = saved.to_json(&catalogue()).unwrap();
        let reopened =
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
        assert_eq!(reopened.devices[0].fields, report.devices[0].fields);
        let blank =
            Workspace::new(Draft::new(version)).with_devices(vec![Spec::new("other", "Other Pen")]);
        let bytes = blank.to_json(&catalogue()).unwrap();
        let reopened =
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
        assert!(reopened.devices[0].fields.is_empty());
    }
}
#[test]
fn exact_legacy_device_schema_remains_compatible_only_for_its_original_overrides() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut spec = Spec::new("old", "Saved Keyboard");
        spec.fields
            .insert("kb_layout".into(), Value::String("us".into()));
        let mut old = Workspace::new(Draft::new(version)).with_devices(vec![spec]);
        old.device_schema_sha256 = Some(LEGACY.into());
        let bytes = old.to_json(&catalogue()).unwrap();
        let reopened =
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
        assert_eq!(reopened.device_schema_sha256.as_deref(), Some(LEGACY));
        assert_eq!(reopened.devices[0].fields.len(), 1);
        let new = Workspace::new(reopened.draft.clone()).with_devices(reopened.devices.clone());
        assert_eq!(
            new.device_schema_sha256.as_deref(),
            Some(device::schema_hash().as_str())
        );
        assert_eq!(new.devices[0].fields, reopened.devices[0].fields);
        let mut forged = reopened.clone();
        forged.devices[0]
            .fields
            .insert("pressure_range_min".into(), Value::Float(0.2));
        assert!(forged.validate(&catalogue()).is_err());
        let mut unknown = reopened.clone();
        unknown.device_schema_sha256 = Some("0".repeat(64));
        assert!(unknown.validate(&catalogue()).is_err());
        let mut missing = reopened;
        missing.device_schema_sha256 = None;
        assert!(missing.validate(&catalogue()).is_err());
    }
}
