use hyprland_editor_core::{
    animation::{self, CurveKind, Operation},
    catalogue, legacy, legacy_animation as codec, Value, AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="# retained café\r\nanimations { # category\r\n enabled = true\r\n bezier = ease,0.12,-3.5,2.5,1 # curve\r\n animation = windows,yes,125,ease,popin 80% # branch\r\n animation = windowsOut,off,ignored-speed,missing-curve,ignored-style # disabled\r\n animation = layers,on,5,forward,fade # unresolved order\r\n bezier = forward,0,0,1,1\r\n animation = layers,on,5,forward,fade\r\n}\r\nsource = missing.conf\r\n";
#[test]
fn native_animation_category_curves_timing_and_disabled_arguments_retain_their_meanings() {
    let report = codec::inspect(SOURCE).unwrap();
    assert_eq!(report.draft.declarations.len(), 5);
    assert_eq!(report.unresolved_count, 1);
    assert_eq!(codec::edited_copy(SOURCE, &report.draft).unwrap(), SOURCE);
    let Operation::Curve(c) = &report.draft.declarations[0].operation else {
        panic!()
    };
    let CurveKind::Bezier { points } = &c.shape else {
        panic!()
    };
    assert_eq!(points[0][1], Value::Float(-3.5));
    assert_eq!(points[1][0], Value::Float(2.5));
    let Operation::Node(n) = &report.draft.declarations[1].operation else {
        panic!()
    };
    assert_eq!(n.speed, Some(Value::Float(125.)));
    assert_eq!(n.bezier.as_deref(), Some("ease"));
    let Operation::Node(n) = &report.draft.declarations[2].operation else {
        panic!()
    };
    assert_eq!(n.enabled, Value::Bool(false));
    assert_eq!(n.speed, Some(Value::String("ignored-speed".into())));
    assert_eq!(n.effective().unwrap().speed, 1.);
    assert!(report.draft.export_lua(BASELINE).is_err());
    assert!(report.draft.validate(AUDITED_MAIN).is_err());
    let mut target = report.draft;
    let Operation::Node(n) = &mut target.declarations[1].operation else {
        panic!()
    };
    n.speed = Some(Value::Float(150.));
    let output = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(output.contains("animation = windows,1,150,ease,popin 80% # branch\r\n"));
    assert!(output.contains("off,ignored-speed,missing-curve,ignored-style # disabled"));
    assert!(output.ends_with("source = missing.conf\r\n"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().draft).unwrap(),
        codec::export(&target).unwrap()
    );
}
#[test]
fn ordered_native_animation_add_duplicate_remove_reorder_and_all_family_copy_reparse() {
    let source="general:border_size = 2\nenv = THEME,Nord\nmonitor = DP-1,preferred,auto,1\nbind = SUPER,Q,exec,application\nworkspace = 1,border:true\nanimations {\n bezier = a,0,0,1,1 # one\n bezier = b,0.1,0.2,0.3,0.4 # two\n animation = windows,1,8,a,slide # node\n}\n";
    let doc = legacy::Document::new(source.into(), &catalogue()).unwrap();
    let mut target = doc.animations.draft.clone();
    target.declarations.swap(0, 1);
    let mut duplicate = target.declarations[0].clone();
    duplicate.id = "copy".into();
    if let Operation::Curve(c) = &mut duplicate.operation {
        c.name = "copy".into();
    }
    target.declarations.push(duplicate);
    let mut draft = doc.report.draft.clone();
    draft
        .values
        .insert("general:border_size".into(), Value::Int(7));
    let output = doc
        .edited_groups(
            legacy::Edits {
                draft: &draft,
                session: &doc.session.draft,
                monitors: &doc.monitors.rules,
                bindings: &doc.bindings.draft,
                workspaces: &doc.workspaces.rules,
                animations: &target,
                gestures: &doc.gestures.draft,
            },
            &catalogue(),
        )
        .unwrap();
    assert!(output.contains("border_size = 7"));
    assert!(output.contains("# one"));
    assert!(output.contains("# two"));
    assert!(output.contains("# node"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().draft).unwrap(),
        codec::export(&target).unwrap()
    );
    let empty = codec::edited_copy(&output, &animation::Draft::default()).unwrap();
    assert!(codec::inspect(&empty).unwrap().draft.is_empty());
    assert!(empty.contains("animations {"));
    assert!(empty.contains("# one"));
    assert!(empty.contains("workspace = 1,border:true"));
    target.declarations.swap(1, 2);
    assert!(target.validate(BASELINE).is_err());
}
#[test]
fn native_numbers_signed_zero_unknown_forms_and_lua_registries_do_not_silently_convert() {
    let source="bezier = x,+0.1,0,1,1\nbezier = y,1e-1,0,1,1\nbezier = z,0.,0,1,1\nanimation = windows,on,8,$curve\nanimation = unknown,1,8,default\nanimations:bezier = opaque,0,0,1,1\n# hyprlang if something\nbezier = conditional,0,0,1,1\n";
    let report = codec::inspect(source).unwrap();
    assert!(report.draft.is_empty());
    assert_eq!(report.unresolved_count, 6);
    assert_eq!(codec::edited_copy(source, &report.draft).unwrap(), source);
    let source = "bezier = zero,-0,0,1,1\n";
    let mut draft = codec::inspect(source).unwrap().draft;
    if let Operation::Curve(c) = &mut draft.declarations[0].operation {
        let CurveKind::Bezier { points } = &mut c.shape else {
            panic!()
        };
        points[0][0] = Value::Float(0.);
    }
    assert!(codec::edited_copy(source, &draft)
        .unwrap()
        .contains("zero,0,0,1,1"));
    for value in [f64::from(f32::MAX), f64::from(f32::MIN_POSITIVE), -2.5, 0.1] {
        let mut d = draft.clone();
        if let Operation::Curve(c) = &mut d.declarations[0].operation {
            let CurveKind::Bezier { points } = &mut c.shape else {
                panic!()
            };
            points[0][0] = Value::Float(value);
        }
        let out = codec::export(&d).unwrap();
        assert_eq!(
            codec::inspect(&out).unwrap().draft.declarations.len(),
            1,
            "{out}"
        );
    }
    let mut d = draft.clone();
    d.declarations[0].legacy = false;
    assert!(codec::export(&d).is_err());
    let mut mixed = codec::inspect("bezier = native,0,0,1,1\n").unwrap().draft;
    let mut node = hyprland_editor_core::lua::parse(
        "hl.animation({leaf='windows',enabled=true,speed=8,bezier='default'})",
        BASELINE,
        &catalogue(),
    )
    .unwrap()
    .animations
    .declarations
    .remove(0);
    if let Operation::Node(n) = &mut node.operation {
        n.bezier = Some("native".into());
    }
    mixed.declarations.push(node);
    assert!(mixed.validate(BASELINE).is_err());
}
#[test]
fn native_animation_project_markers_preserve_intentional_empty_views() {
    use hyprland_editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        workspace::{SourceLanguage, Workspace},
        Draft,
    };
    let source = "bezier = ease,0,0,1,1\n";
    let hash = hyprland_editor_core::lua::source_hash(source);
    let id = SourceIdentity {
        selected_path: b"/never-opened/config.conf".to_vec(),
        resolved_path: None,
        links: vec![],
        device: 1,
        inode: 2,
        owner: 1000,
        mode: 0o100600,
        size: source.len() as u64,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
        sha256: hash.clone(),
        recorded_ownership: RecordedOwnership::UserOwned,
    };
    let mut w = Workspace::new(Draft::new(BASELINE))
        .with_root(source.into(), id)
        .with_animations(codec::inspect(source).unwrap().draft);
    w.sources[0].language = SourceLanguage::Legacy;
    assert!(w.validate(&catalogue()).is_err());
    w.legacy_animation_origin_sha256 = Some(hash);
    w.legacy_animation_schema_sha256 = Some(codec::schema_hash());
    w.validate(&catalogue()).unwrap();
    w.animations.declarations.clear();
    let b = w.to_json(&catalogue()).unwrap();
    let r = Workspace::from_json(std::str::from_utf8(&b).unwrap(), &catalogue()).unwrap();
    assert!(r.animations.is_empty());
    assert!(r.legacy_animation_origin_sha256.is_some());
    w.legacy_animation_schema_sha256 = Some("0".repeat(64));
    assert!(w.validate(&catalogue()).is_err());
}
