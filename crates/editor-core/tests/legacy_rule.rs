use hyprland_editor_core::{
    catalogue, legacy, legacy_rule as codec,
    rule::{self, Effect, EffectValue, Kind, Rule},
    Value, BASELINE,
};
const SOURCE:&str="# retain café\r\nwindowrule { # browser\r\n name = browser\r\n match:class = ^firefox$ # match\r\n opacity = 0.8 # prior\r\n opacity = 0.9 override # last\r\n float = yes # native switch\r\n enable = 2 # raw enable\r\n}\r\nlayerrule {\r\n name = panel\r\n match:namespace = ^waybar$\r\n enable = 0\r\n blur = on\r\n}\r\nwindowrule = match:class ^foot$, tag +a, tag +b, float 1 # ordered\r\nsource = missing.conf\r\n";
#[test]
fn all_native_rule_registrations_keep_literal_values_and_separate_lua_semantics() {
    let profile: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/legacy-rules.json")).unwrap();
    assert_eq!(profile["matches"].as_array().unwrap().len(), 18);
    assert_eq!(profile["matches"], serde_json::json!(codec::properties()));
    for kind in [Kind::Window, Kind::Layer] {
        let fields = rule::catalogue(kind, BASELINE);
        assert_eq!(fields.len(), if kind == Kind::Window { 57 } else { 10 });
        let mut registered = profile[if kind == Kind::Window {
            "window_effects"
        } else {
            "layer_effects"
        }]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>();
        registered.sort();
        let mut catalogue = fields
            .iter()
            .map(|s| s.lua_key.as_str())
            .collect::<Vec<_>>();
        catalogue.sort();
        assert_eq!(registered, catalogue);
        for f in fields {
            let mut r = Rule::new("rule", kind);
            r.legacy = Some(codec::Native {
                block: true,
                enable: Some(i64::MAX),
            });
            r.name = Some("test".into());
            r.effects.push(Effect {
                key: f.lua_key.clone(),
                value: EffectValue::Literal(Value::String("native opaque value".into())),
            });
            let out = codec::encode(&r).unwrap();
            let parsed = codec::inspect(&out).unwrap();
            assert_eq!(parsed.rules.len(), 1, "{} {out}", f.lua_key);
            assert!(rule::same_declarations(&[r.clone()], &parsed.rules));
            assert!(r.export_lua(BASELINE).is_err());
        }
    }
    for property in codec::properties() {
        for kind in ["windowrule", "layerrule"] {
            let s = format!("{kind} {{\n name = one\n match:{property} = -2 native\n}}\n");
            let r = codec::inspect(&s).unwrap();
            assert_eq!(r.rules.len(), 1);
            assert_eq!(
                r.rules[0].predicates[0].value,
                Value::String("-2 native".into())
            );
        }
    }
}
#[test]
fn native_block_fields_keep_comments_duplicates_enable_presence_and_empty_values() {
    let before = codec::inspect(SOURCE).unwrap();
    assert_eq!(before.rules.len(), 3);
    assert_eq!(before.rules[0].legacy.as_ref().unwrap().enable, Some(2));
    assert_eq!(before.rules[1].legacy.as_ref().unwrap().enable, Some(0));
    assert_eq!(codec::edited_copy(SOURCE, &before.rules).unwrap(), SOURCE);
    let mut target = before.rules;
    target[0]
        .effects
        .iter_mut()
        .find(|e| e.key == "opacity")
        .unwrap()
        .value = EffectValue::Literal(Value::String("0.5 0.25 override".into()));
    target[0].effects.retain(|e| e.key != "float");
    target[1].legacy.as_mut().unwrap().enable = None;
    target[0].predicates[0].value = Value::String(String::new());
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(out.contains("opacity = 0.8 # prior\r\n"));
    assert!(out.contains("opacity = 0.5 0.25 override # last\r\n"));
    assert!(out.contains("# native switch\r\n"));
    assert!(!out.contains("float = yes"));
    assert!(!out.contains("enable = 0"));
    assert!(out.contains("match:class =  # match"));
    assert!(out.ends_with("source = missing.conf\r\n"));
    assert!(rule::same_declarations(
        &codec::inspect(&out).unwrap().rules,
        &target
    ));
}
#[test]
fn inline_effect_duplicates_keep_order_and_refuse_implicit_block_collapse() {
    let mut r = codec::inspect("windowrule = tag +a, match:class ^foot$, tag +b, tag -a\n")
        .unwrap()
        .rules
        .remove(0);
    assert_eq!(r.effects.len(), 3);
    let out = codec::encode(&r).unwrap();
    assert!(out.ends_with("tag +a, tag +b, tag -a"));
    let original = r.clone();
    r.effects.swap(0, 1);
    assert!(!rule::same_declarations(&[original], &[r.clone()]));
    r.legacy.as_mut().unwrap().block = true;
    r.name = Some("changed".into());
    assert!(r.validate(BASELINE).is_err());
}
#[test]
fn source_copy_reorders_converts_adds_removes_and_rechecks_all_nine_groups() {
    let source=format!("{SOURCE}general:border_size = 2\nenv = THEME,Nord\nmonitor = DP-1,preferred,auto,1\nbind = SUPER,Q,exec,app\nworkspace = 1,border:true\nbezier = ease,0,0,1,1\nanimation = windows,1,8,ease\ngesture = 3,left,workspace\ndevice {{\n name = pointer\n sensitivity = 0.2\n}}\n");
    let c = catalogue();
    let doc = legacy::Document::new(source, &c).unwrap();
    let mut rules = doc.rules.rules.clone();
    rules.swap(0, 1);
    let mut added = rules[0].clone();
    added.id = "added".into();
    added.name = Some("other".into());
    rules.push(added);
    rules.remove(2);
    let out = doc
        .edited_groups(
            legacy::Edits {
                draft: &doc.report.draft,
                session: &doc.session.draft,
                monitors: &doc.monitors.rules,
                bindings: &doc.bindings.draft,
                workspaces: &doc.workspaces.rules,
                animations: &doc.animations.draft,
                gestures: &doc.gestures.draft,
                devices: &doc.devices.devices,
                rules: &rules,
            },
            &c,
        )
        .unwrap();
    assert!(rule::same_declarations(
        &codec::inspect(&out).unwrap().rules,
        &rules
    ));
    assert!(out.contains("# ordered"));
    assert!(out.contains("# browser"));
}
#[test]
fn unsupported_rule_syntax_is_retained_and_unresolved_detail_is_bounded() {
    for s in [
        "windowrulev2 = float,class:foo\n",
        "windowrule {\n name = $dynamic\n float = yes\n}\n",
        "layerrule {\n name = one\n mystery = x\n}\n",
        "windowrule = unknown value\n",
    ] {
        let r = codec::inspect(s).unwrap();
        assert!(r.rules.is_empty());
        assert_eq!(codec::edited_copy(s, &[]).unwrap(), s);
    }
    let s = "windowrule = unknown value\n".repeat(5000);
    let r = codec::inspect(&s).unwrap();
    assert_eq!(r.unresolved_count, 5000);
    assert_eq!(r.unresolved.len(), 4096);
}
#[test]
fn native_rule_source_markers_preserve_empty_targets_and_refuse_missing_provenance() {
    use hyprland_editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        workspace::{SourceLanguage, Workspace},
    };
    let c = catalogue();
    let doc = legacy::Document::new(SOURCE.into(), &c).unwrap();
    let origin = SourceIdentity {
        selected_path: b"/never-read/rules.conf".to_vec(),
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
        sha256: hyprland_editor_core::lua::source_hash(SOURCE),
        recorded_ownership: RecordedOwnership::UserOwned,
    };
    let mut w = Workspace::new(doc.report.draft)
        .with_root(SOURCE.into(), origin)
        .with_rules(doc.rules.rules);
    w.sources[0].language = SourceLanguage::Legacy;
    assert!(w.to_json(&c).is_err());
    w.legacy_rule_origin_sha256 = Some(hyprland_editor_core::lua::source_hash(SOURCE));
    w.legacy_rule_schema_sha256 = Some(codec::schema_hash());
    let saved = w.to_json(&c).unwrap();
    assert!(Workspace::from_json(std::str::from_utf8(&saved).unwrap(), &c).is_ok());
    w.rules.clear();
    assert!(
        Workspace::from_json(std::str::from_utf8(&w.to_json(&c).unwrap()).unwrap(), &c)
            .unwrap()
            .rules
            .is_empty()
    );
    w.legacy_rule_schema_sha256 = Some("0".repeat(64));
    assert!(w.to_json(&c).is_err());
}
