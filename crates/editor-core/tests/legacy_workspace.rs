use hyprland_editor_core::{
    catalogue, legacy, legacy_workspace as codec,
    workspace::{SourceLanguage, Workspace},
    workspace_rule::{self, Rule},
    Value, AUDITED_MAIN, BASELINE,
};
const SOURCE: &str = "# retained café\r\nworkspace = name:work,monitor:DP-1,default:true,persistent:true,gapsin:2 4,gapsout:1 2 3 4,bordersize:3,border:false,shadow:0,rounding:off,decorate:on,defaultName:Work,on-created-empty:launch application,layoutopt:orientation:center,layout:master,animation:slide # all fields\r\nworkspace = name:work,bordersize:5,layoutopt:custom:one:two # merge\r\nworkspace = $unresolved,monitor:$output # unresolved\r\nsource = missing.conf\r\n";
#[test]
fn workspace_fields_inversion_strings_comments_and_repeated_selectors_round_trip() {
    let report = codec::inspect(SOURCE).unwrap();
    assert_eq!(report.rules.len(), 2);
    assert_eq!(report.unresolved_count, 1);
    let r = &report.rules[0];
    assert_eq!(r.fields.len(), 14);
    assert!(r.legacy);
    assert_eq!(r.fields["gaps_in"], Value::CssGap([2, 4, 2, 4]));
    assert_eq!(r.fields["gaps_out"], Value::CssGap([1, 2, 3, 4]));
    for key in ["no_border", "no_shadow", "no_rounding", "decorate"] {
        assert_eq!(r.fields[key], Value::Bool(true));
    }
    assert_eq!(
        r.layout_options["orientation"],
        Value::String("center".into())
    );
    assert_eq!(
        report.rules[1].layout_options["custom"],
        Value::String("one:two".into())
    );
    assert_eq!(codec::edited_copy(SOURCE, &report.rules).unwrap(), SOURCE);
    let mut rules = report.rules;
    rules[0]
        .fields
        .insert("no_border".into(), Value::Bool(false));
    rules[0]
        .fields
        .insert("default_name".into(), Value::String("New # Name".into()));
    let output = codec::edited_copy(SOURCE, &rules).unwrap();
    assert!(output.contains("border:true"));
    assert!(output.contains("# all fields\r\n"));
    assert!(output.ends_with("source = missing.conf\r\n"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&rules).unwrap()
    );
    assert_eq!(
        workspace_rule::consolidate(&rules)[0].fields["border_size"],
        Value::Int(5)
    );
}
#[test]
fn ordered_workspace_copy_supports_add_duplicate_remove_reorder_and_combined_families() {
    let source=format!("{SOURCE}general:border_size = 2\r\nenv = THEME,Nord\r\nmonitor = DP-1,preferred,auto,1\r\nbind = SUPER,Q,exec,application\r\n");
    let doc = legacy::Document::new(source, &catalogue()).unwrap();
    let mut rules = doc.workspaces.rules.clone();
    rules.swap(0, 1);
    rules.remove(1);
    let mut r = rules[0].clone();
    r.id = "copy".into();
    rules.push(r);
    let mut r = Rule::new("new");
    r.legacy = true;
    r.selector = "special:notes".into();
    rules.push(r);
    let mut draft = doc.report.draft.clone();
    draft
        .values
        .insert("general:border_size".into(), Value::Int(7));
    let output = doc
        .edited_with_workspaces(
            &draft,
            &doc.session.draft,
            &doc.monitors.rules,
            &doc.bindings.draft,
            &rules,
            &catalogue(),
        )
        .unwrap();
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&rules).unwrap()
    );
    assert!(output.contains("general:border_size = 7"));
    assert!(output.contains("bind = SUPER,Q,exec,application"));
    let empty = codec::edited_copy(&output, &[]).unwrap();
    assert_eq!(codec::inspect(&empty).unwrap().rules.len(), 0);
    assert!(empty.contains("# all fields"));
    assert!(empty.contains("workspace = $unresolved"));
}
#[test]
fn unsupported_syntax_and_native_substring_precedence_never_silently_change_fields() {
    let source="workspace = 1,float_gaps:2\nworkspace = 2,defaultname:wrong-case\nworkspace = 3,layoutopt:broken\nworkspace = 4,defaultName:contains gapsin:4\nworkspace = 5,layout:{{1+1}}\nworkspace = 6,monitor:$output\n# hyprlang noerror true\nworkspace = 7,border:true\n";
    let report = codec::inspect(source).unwrap();
    assert!(report.rules.is_empty());
    assert_eq!(report.unresolved_count, 7);
    assert_eq!(codec::edited_copy(source, &[]).unwrap(), source);
    let mut r = Rule::new("native");
    r.legacy = true;
    r.fields.insert(
        "default_name".into(),
        Value::String("contains gapsin:4".into()),
    );
    assert!(codec::encode(&r).is_err());
    r.fields.clear();
    r.fields.insert(
        "default_name".into(),
        Value::String("name,with,commas".into()),
    );
    assert!(codec::encode(&r).is_ok());
    assert!(r.export_lua(BASELINE).is_err());
    assert!(r.validate(AUDITED_MAIN).is_err());
    r.enabled = Some(false);
    assert!(r.validate(BASELINE).is_err());
    r.enabled = None;
    r.layout_options.insert("x".into(), Value::Int(1));
    assert!(r.validate(BASELINE).is_err());
    r.layout_options.clear();
    r.legacy = false;
    assert!(codec::encode(&r).is_err());
}
#[test]
fn native_workspace_projects_preserve_empty_targets_and_require_source_and_codec_markers() {
    use hyprland_editor_core::{
        lua::source_hash,
        source::{RecordedOwnership, SourceIdentity},
    };
    let hash = source_hash(SOURCE);
    let mut project = Workspace::new(hyprland_editor_core::Draft::new(BASELINE))
        .with_workspace_rules(codec::inspect(SOURCE).unwrap().rules);
    project.root_source = Some("root".into());
    project
        .sources
        .push(hyprland_editor_core::workspace::SourceSnapshot {
            id: "root".into(),
            language: SourceLanguage::Legacy,
            content: SOURCE.into(),
            origin: SourceIdentity {
                selected_path: b"/never-opened/config.conf".to_vec(),
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
            },
        });
    assert!(project.validate(&catalogue()).is_err());
    project.legacy_workspace_origin_sha256 = Some(hash);
    project.legacy_workspace_schema_sha256 = Some(codec::schema_hash());
    project.validate(&catalogue()).unwrap();
    project.workspace_rules.clear();
    let bytes = project.to_json(&catalogue()).unwrap();
    let reopened =
        Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
    assert!(reopened.workspace_rules.is_empty());
    assert!(reopened.legacy_workspace_origin_sha256.is_some());
    project.legacy_workspace_schema_sha256 = Some("0".repeat(64));
    assert!(project.validate(&catalogue()).is_err());
}
