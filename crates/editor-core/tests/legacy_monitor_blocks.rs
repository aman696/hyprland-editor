use hyprland_editor_core::{
    catalogue, legacy, legacy_monitor as codec,
    monitor::{LegacyOperation as Op, Output, Rule},
    Value, BASELINE,
};
const SOURCE:&str="# display\r\nmonitor = HDMI-A-1,preferred,auto,1\r\nmonitorv2 { # block\r\n    output = desc:Main screen # target\r\n    mode = 2560x1440@144 # resolution\r\n    supports_hdr = 1 # HDR\r\n    supports_wide_color = 1\r\n    sdr_eotf = 1\r\n    sdrbrightness = -0.0\r\n    sdr_min_luminance = 0.2\r\n    sdr_max_luminance = 100\r\n    min_luminance = 0.01\r\n    max_luminance = 1000\r\n    max_avg_luminance = 400\r\n    addreserved = 1,2,3,4 # space\r\n    icc = # empty profile\r\n} # close\r\nsource = missing.conf\r\n";
#[test]
fn monitor_blocks_populate_hdr_luminance_transfer_and_reserved_controls_without_losing_text() {
    let before = codec::inspect(SOURCE).unwrap();
    assert_eq!(before.rules.len(), 2);
    assert_eq!(before.unresolved_count, 0);
    let rule = &before.rules[1];
    assert_eq!(rule.legacy_operation, Some(Op::Block));
    assert_eq!(rule.output, Output::Description("Main screen".into()));
    assert_eq!(rule.fields["reserved"], Value::CssGap([1, 4, 2, 3]));
    assert_eq!(rule.fields["icc"], Value::String(String::new()));
    assert!(
        matches!(rule.fields["sdrbrightness"],Value::Float(v) if v.to_bits()==(-0.0f64).to_bits())
    );
    assert_eq!(codec::edited_copy(SOURCE, &before.rules).unwrap(), SOURCE);
    let mut target = before.rules.clone();
    let rule = &mut target[1];
    rule.output = Output::Description("New screen#1".into());
    rule.fields.insert("max_luminance".into(), Value::Int(1200));
    rule.fields.remove("supports_hdr");
    rule.fields.insert("vrr".into(), Value::Int(2));
    rule.fields
        .insert("sdrbrightness".into(), Value::Float(0.0));
    let output = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(output.contains("    max_luminance = 1200\r\n"));
    assert!(output.contains("mode = 2560x1440@144 # resolution\r\n"));
    assert!(output.contains("    # HDR\r\n"));
    assert!(output.contains("sdrbrightness = 0\r\n"));
    assert!(output.contains("    vrr = 2\r\n} # close"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&target).unwrap()
    );
    assert!(output.ends_with("source = missing.conf\r\n"));
}
#[test]
fn block_add_duplicate_remove_reorder_and_operation_conversion_retain_comments() {
    let before = codec::inspect(SOURCE).unwrap();
    let mut rules = before.rules.clone();
    let mut duplicate = rules[1].clone();
    duplicate.id = "duplicate".into();
    duplicate.output = Output::Connector("DP-2".into());
    rules.push(duplicate);
    let output = codec::edited_copy(SOURCE, &rules).unwrap();
    assert_eq!(codec::inspect(&output).unwrap().rules.len(), 3);
    rules.swap(0, 1);
    let output = codec::edited_copy(SOURCE, &rules).unwrap();
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&rules).unwrap()
    );
    for comment in [
        "# block",
        "# target",
        "# resolution",
        "# HDR",
        "# space",
        "# empty profile",
        "# close",
    ] {
        assert!(output.contains(comment), "{comment}");
    }
    let removed = codec::edited_copy(SOURCE, &[]).unwrap();
    assert!(codec::inspect(&removed).unwrap().rules.is_empty());
    for comment in ["# block", "# resolution", "# HDR", "# close"] {
        assert!(removed.contains(comment));
    }
    let mut transform = Rule::new("transform");
    transform.output = Output::Connector("DP-1".into());
    transform.legacy_operation = Some(Op::Transform);
    transform.fields.insert("transform".into(), Value::Int(1));
    rules = vec![before.rules[0].clone(), transform];
    let output = codec::edited_copy(SOURCE, &rules).unwrap();
    assert!(output.contains("monitor = DP-1,transform,1"));
    assert!(output.contains("# HDR"));
    assert_eq!(codec::inspect(&output).unwrap().rules.len(), 2);
}
#[test]
fn repeated_output_blocks_keep_separate_partial_declarations_and_duplicate_fields() {
    let source="monitorv2 {\n output = DP-1\n max_luminance = 500 # first\n max_luminance = 600 # last\n}\nmonitorv2 {\n output = DP-1\n supports_hdr = yes\n}\n";
    let report = codec::inspect(source).unwrap();
    assert_eq!(report.rules.len(), 2);
    assert_eq!(report.rules[0].fields["max_luminance"], Value::Int(600));
    assert!(!report.rules[1].fields.contains_key("max_luminance"));
    let mut target = report.rules;
    target[0]
        .fields
        .insert("max_luminance".into(), Value::Int(700));
    let output = codec::edited_copy(source, &target).unwrap();
    assert!(output.contains("max_luminance = 500 # first"));
    assert!(output.contains("max_luminance = 700 # last"));
    target[0].fields.remove("max_luminance");
    let removed = codec::edited_copy(source, &target).unwrap();
    assert!(!removed.contains("max_luminance"));
    assert!(removed.contains("# first") && removed.contains("# last"));
}
#[test]
fn unsupported_blocks_stay_unresolved_and_do_not_absorb_following_valid_blocks() {
    let source="device {\n monitorv2 {\n output = nested\n }\n}\nmonitorv2 {\n output = $dynamic\n}\nmonitorv2 {\n output = DP-1\n future_field = value\n}\nmonitorv2 {\n output = DP-2\n max_luminance = 1000\n}\n";
    let report = codec::inspect(source).unwrap();
    assert_eq!(report.rules.len(), 1);
    assert_eq!(report.unresolved_count, 3);
    let removed = codec::edited_copy(source, &[]).unwrap();
    for line in [
        "output = nested",
        "output = $dynamic",
        "future_field = value",
    ] {
        assert!(removed.contains(line));
    }
    assert!(!removed.contains("output = DP-2"));
}
#[test]
fn block_codec_combines_with_scalar_session_edits_and_preserves_previous_schema_acceptance() {
    let source = format!("{SOURCE}general:border_size = 2\nenv = THEME,old\n");
    let doc = legacy::Document::new(source, &catalogue()).unwrap();
    let mut scalar = doc.report.draft.clone();
    scalar
        .values
        .insert("general:border_size".into(), Value::Int(4));
    let output = doc
        .edited_with_monitors(
            &scalar,
            &doc.session.draft,
            &doc.monitors.rules,
            &catalogue(),
        )
        .unwrap();
    assert!(output.contains("border_size = 4"));
    assert!(output.contains("# HDR"));
    let old = hyprland_editor_core::lua::source_hash(include_str!(
        "../../../data/legacy-monitors-v1.json"
    ));
    assert!(codec::accepts_schema(&old, &doc.monitors.rules[..1]));
    assert!(!codec::accepts_schema(&old, &doc.monitors.rules));
    assert!(doc.monitors.rules[1].export_lua(BASELINE).is_err());
}
