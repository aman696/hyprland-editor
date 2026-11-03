use hyprland_editor_core::{
    catalogue,
    device::{self, Spec},
    legacy, legacy_device as codec, Value, AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="# retained café\r\ndevice { # first\r\n name = test-keyboard\r\n kb_layout = us,ru # layouts\r\n kb_variant = ,phonetic\r\n tap-to-click = true # alias\r\n repeat_rate = 25 # prior rate\r\n repeat_rate = 30 # final rate\r\n enabled = 2 # raw integer\r\n scroll_factor = -1 # native range\r\n region_size = -2 3 # native vector\r\n} # close first\r\ndevice {\r\n name = test-keyboard\r\n natural_scroll = yes\r\n}\r\nsource = missing.conf\r\n";
#[test]
fn every_native_device_registration_and_raw_range_has_a_lossless_graphical_model() {
    let fields = device::catalogue(BASELINE);
    assert_eq!(fields.len(), 47);
    for field in fields {
        let mut s = Spec::new("native", "fixture-device");
        s.legacy = true;
        let v = match field.kind.as_str() {
            "Bool" => Value::Bool(true),
            "Int" => Value::Int(720),
            "Float" => Value::Float(-2.5),
            "Vec2" => Value::Vec2([-2.5, 3.5]),
            _ => Value::String("native literal".into()),
        };
        s.fields.insert(field.lua_key.clone(), v);
        let out = codec::encode(&s).unwrap();
        let r = codec::inspect(&out).unwrap();
        assert_eq!(r.devices.len(), 1, "{} {out}", field.lua_key);
        assert!(device::same_declarations(&[s.clone()], &r.devices));
        assert!(out.contains(&format!("{} =", codec::native_key(&field.lua_key))));
        assert!(s.export_lua(BASELINE).is_err());
        assert!(s.validate(AUDITED_MAIN).is_err());
        if field.kind == "Bool" {
            s.fields.insert(field.lua_key.clone(), Value::Int(i64::MAX));
            let out = codec::encode(&s).unwrap();
            let r = codec::inspect(&out).unwrap();
            assert_eq!(r.devices[0].fields[&field.lua_key], Value::Int(i64::MAX));
        }
    }
}
#[test]
fn device_field_updates_keep_aliases_duplicates_comments_native_values_and_unset_presence() {
    let r = codec::inspect(SOURCE).unwrap();
    assert_eq!(r.devices.len(), 2);
    assert!(r.devices.iter().all(|s| s.legacy));
    assert_eq!(r.devices[0].fields["enabled"], Value::Int(2));
    assert_eq!(r.devices[0].fields["scroll_factor"], Value::Float(-1.));
    assert_eq!(r.devices[0].fields["region_size"], Value::Vec2([-2., 3.]));
    assert_eq!(codec::edited_copy(SOURCE, &r.devices).unwrap(), SOURCE);
    let mut target = r.devices;
    target[0]
        .fields
        .insert("repeat_rate".into(), Value::Int(500));
    target[0].fields.remove("tap_to_click");
    target[0]
        .fields
        .insert("sensitivity".into(), Value::Float(0.75));
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(out.contains("repeat_rate = 25 # prior rate\r\n"));
    assert!(out.contains("repeat_rate = 500 # final rate\r\n"));
    assert!(out.contains("# alias\r\n"));
    assert!(!out.contains("tap-to-click ="));
    assert!(out.contains("enabled = 2 # raw integer\r\n"));
    assert!(out.contains("# close first\r\n"));
    assert!(out.ends_with("source = missing.conf\r\n"));
    assert_eq!(
        codec::export(&codec::inspect(&out).unwrap().devices).unwrap(),
        codec::export(&target).unwrap()
    );
    target[0].fields.remove("repeat_rate");
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(!out.contains("repeat_rate ="));
    assert!(out.contains("# prior rate"));
    assert!(out.contains("# final rate"));
}
#[test]
fn native_device_order_duplicates_removal_and_all_eight_source_groups_reparse_together() {
    let source=format!("{SOURCE}general:border_size = 2\nenv = THEME,Nord\nmonitor = DP-1,preferred,auto,1\nbind = SUPER,Q,exec,app\nworkspace = 1,border:true\nbezier = ease,0,0,1,1\nanimation = windows,1,8,ease\ngesture = 3,left,workspace\n");
    let doc = legacy::Document::new(source, &catalogue()).unwrap();
    let mut target = doc.devices.devices.clone();
    target.swap(0, 1);
    let mut extra = target[0].clone();
    extra.id = "copy".into();
    extra.name = "other-device".into();
    target.push(extra);
    let mut draft = doc.report.draft.clone();
    draft
        .values
        .insert("general:border_size".into(), Value::Int(7));
    let out = doc
        .edited_groups(
            legacy::Edits {
                draft: &draft,
                session: &doc.session.draft,
                monitors: &doc.monitors.rules,
                bindings: &doc.bindings.draft,
                workspaces: &doc.workspaces.rules,
                animations: &doc.animations.draft,
                gestures: &doc.gestures.draft,
                devices: &target,
                rules: &doc.rules.rules,
            },
            &catalogue(),
        )
        .unwrap();
    assert_eq!(
        codec::export(&codec::inspect(&out).unwrap().devices).unwrap(),
        codec::export(&target).unwrap()
    );
    assert!(out.contains("border_size = 7"));
    assert!(out.contains("# first"));
    assert!(out.contains("# layouts"));
    let empty = codec::edited_copy(&out, &[]).unwrap();
    assert!(codec::inspect(&empty).unwrap().devices.is_empty());
    assert!(empty.contains("# layouts"));
    assert!(empty.contains("gesture = 3,left,workspace"));
}
#[test]
fn unsupported_device_blocks_stay_whole_and_do_not_hide_following_supported_blocks() {
    let source="device {\n name = $device\n enabled = true\n}\ndevice {\n enabled = true\n name = late\n}\ndevice {\n name = duplicate\n name = second\n}\ndevice {\n name = unknown\n tap_to_click = true\n}\ndevice {\n name = nested\n child {\n enabled = false\n }\n}\ndevice {\n name = valid\n enabled = false\n}\n";
    let r = codec::inspect(source).unwrap();
    assert_eq!(r.unresolved_count, 5);
    assert_eq!(r.devices.len(), 1);
    let out = codec::edited_copy(source, &[]).unwrap();
    assert!(out.contains("name = $device"));
    assert!(out.contains("tap_to_click = true"));
    assert!(!out.contains("name = valid"));
    let mut s = Spec::new("bad", "retained");
    s.legacy = true;
    s.fields
        .insert("sensitivity".into(), Value::Float(f64::MAX));
    assert!(s.validate(BASELINE).is_err());
    s.fields.clear();
    s.fields.insert(
        "kb_file".into(),
        Value::String("/never-read/private/keymap".into()),
    );
    assert!(codec::encode(&s).is_ok());
    s.legacy = false;
    assert!(codec::encode(&s).is_err());
    let many = "device {\n}\n".repeat(5000);
    let r = codec::inspect(&many).unwrap();
    assert_eq!(r.unresolved_count, 5000);
    assert_eq!(r.unresolved.len(), 4096);
    assert_eq!(codec::edited_copy(&many, &[]).unwrap(), many);
}
#[test]
fn native_device_project_markers_retain_empty_targets_and_refuse_old_schema_authority() {
    use hyprland_editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        workspace::{SourceLanguage, Workspace},
        Draft,
    };
    let hash = hyprland_editor_core::lua::source_hash(SOURCE);
    let origin = SourceIdentity {
        selected_path: b"/never-read/devices.conf".to_vec(),
        resolved_path: None,
        links: vec![],
        device: 1,
        inode: 2,
        owner: 1000,
        mode: 0o100600,
        size: SOURCE.len() as u64,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
        sha256: hash.clone(),
        recorded_ownership: RecordedOwnership::UserOwned,
    };
    let mut w = Workspace::new(Draft::new(BASELINE))
        .with_root(SOURCE.into(), origin)
        .with_devices(codec::inspect(SOURCE).unwrap().devices);
    w.sources[0].language = SourceLanguage::Legacy;
    assert!(w.validate(&catalogue()).is_err());
    w.legacy_device_origin_sha256 = Some(hash);
    w.legacy_device_schema_sha256 = Some(codec::schema_hash());
    w.validate(&catalogue()).unwrap();
    w.device_schema_sha256 =
        Some("f555d3d357acca4e74b4472f9a7fc441c6f347b8a2b47c8b6d466dbde45966b7".into());
    assert!(w.validate(&catalogue()).is_err());
    w.device_schema_sha256 = Some(device::schema_hash());
    w.devices.clear();
    let bytes = w.to_json(&catalogue()).unwrap();
    let r = Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
    assert!(r.devices.is_empty());
    assert!(r.legacy_device_origin_sha256.is_some());
}
