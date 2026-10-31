use hyprland_editor_core::{
    catalogue,
    gesture::{self, GestureAction},
    legacy, legacy_gesture as codec, Value, AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="# touchpad café\r\ngestures {\r\n gesturepp = 3,HORIZ,mod:ALT,mod:SUPER CTRL,scale:-2,workspace # matching\r\n gesture = 4,pinch,cursorZoom,2.5,mult # zoom order\r\n gesture = 3,L,dispatcher,exec,launch app\\,with commas # dispatcher\r\n gesture = 4,down,scrollMove # spelling\r\n gesture = $fingers,up,close # unresolved\r\n}\r\nsource = missing.conf\r\n";
#[test]
fn native_gesture_flags_modifiers_scale_clamp_builtin_spelling_and_dispatcher_arguments_round_trip()
{
    let r = codec::inspect(SOURCE).unwrap();
    assert_eq!(r.draft.declarations.len(), 4);
    assert_eq!(r.unresolved_count, 1);
    assert_eq!(codec::edited_copy(SOURCE, &r.draft).unwrap(), SOURCE);
    let s = &r.draft.declarations[0];
    assert_eq!(s.mods, Some(Value::String("SUPER CTRL".into())));
    assert_eq!(s.scale, Some(Value::Float(-2.)));
    assert_eq!(s.identity(BASELINE).unwrap().scale_bits, 0.1f32.to_bits());
    assert!(s.identity(BASELINE).unwrap().disable_inhibit);
    let s = &r.draft.declarations[1];
    assert_eq!(s.zoom_level, Some(Value::String("2.5".into())));
    assert_eq!(s.mode, Some(Value::String("mult".into())));
    assert!(
        matches!(&r.draft.declarations[2].action,GestureAction::NativeDispatcher{name,argument} if name=="exec"&&argument=="launch app,with commas")
    );
    assert!(r.draft.export_lua(BASELINE).is_err());
    assert!(r.draft.validate(AUDITED_MAIN).is_err());
    let mut target = r.draft;
    target.declarations[0].scale = Some(Value::Float(20.));
    target.declarations[2].action = GestureAction::NativeDispatcher {
        name: "pluginAction".into(),
        argument: "new app, with,comma,,tail,".into(),
    };
    let output = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(output.contains("gesturep = 3,HORIZ,mod:SUPER CTRL,scale:20,workspace # matching"));
    assert!(
        output.contains(r"new app\\, with\\,comma\\,\\,tail\\,"),
        "{output}"
    );
    assert!(output.contains("# dispatcher\r\n"));
    assert!(output.ends_with("source = missing.conf\r\n"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().draft).unwrap(),
        codec::export(&target).unwrap()
    );
    assert_eq!(
        target.declarations[0]
            .identity(BASELINE)
            .unwrap()
            .scale_bits,
        10f32.to_bits()
    );
}
#[test]
fn native_gesture_registration_unset_add_duplicate_order_and_all_family_copy_remain_inert() {
    let source="general:border_size = 2\nenv = THEME,Nord\nmonitor = DP-1,preferred,auto,1\nbind = SUPER,Q,exec,application\nworkspace = 1,border:true\nbezier = ease,0,0,1,1\nanimation = windows,1,8,ease\ngesturep = 3,horizontal,scale:20,workspace # add\ngesturep = 3,horizontal,scale:10,unset # remove\n";
    let doc = legacy::Document::new(source.into(), &catalogue()).unwrap();
    let outcomes = doc.gestures.draft.registrations(BASELINE).unwrap();
    assert_eq!(outcomes[1].result, Ok(Some(0)));
    let mut target = doc.gestures.draft.clone();
    target.declarations.remove(1);
    let mut copy = target.declarations[0].clone();
    copy.id = "new-gesture".into();
    copy.fingers = Value::Int(4);
    copy.action = GestureAction::Builtin("special".into());
    copy.workspace_name = Some(Value::String("notes,private".into()));
    target.declarations.push(copy);
    target.declarations.swap(0, 1);
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
                animations: &doc.animations.draft,
                gestures: &target,
            },
            &catalogue(),
        )
        .unwrap();
    assert!(output.contains("border_size = 7"));
    assert!(output.contains("# add"));
    assert!(output.contains("# remove"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().draft).unwrap(),
        codec::export(&target).unwrap()
    );
    let empty = codec::edited_copy(&output, &gesture::Draft::default()).unwrap();
    assert!(codec::inspect(&empty).unwrap().draft.is_empty());
    assert!(empty.contains("# add"));
    assert!(empty.contains("animation = windows,1,8,ease"));
}
#[test]
fn native_unknown_syntax_parameter_boundaries_and_lua_workflows_do_not_silently_convert() {
    let source="gesturez = 3,left,workspace\ngesture = 1,left,workspace\ngesture = 3,unknown,close\ngesture = 3,left,scale:NaN,workspace\ngesture = 3,left,unknown\ngesture = 3,left,workspace,extra\ngesture = 3,left,dispatcher,exec,$command\n# hyprlang if private\ngesture = 3,up,close\n";
    let r = codec::inspect(source).unwrap();
    assert!(r.draft.is_empty());
    assert_eq!(r.unresolved_count, 8);
    assert_eq!(codec::edited_copy(source, &r.draft).unwrap(), source);
    for (action, args) in [
        ("special", ",notes"),
        ("float", ",tile"),
        ("fullscreen", ",maximize"),
        ("cursorZoom", ",2.5,live"),
        ("scrollMove", ""),
        ("resize", ""),
        ("move", ""),
        ("close", ""),
    ] {
        let source = format!("gesture = 3,up,{action}{args}\n");
        let d = codec::inspect(&source).unwrap().draft;
        assert_eq!(d.declarations.len(), 1, "{source}");
        assert_eq!(
            codec::export(&codec::inspect(&codec::export(&d).unwrap()).unwrap().draft).unwrap(),
            codec::export(&d).unwrap()
        );
    }
    let lua = hyprland_editor_core::lua::parse(
        "hl.gesture{fingers=3,direction='left',action='workspace'}",
        BASELINE,
        &catalogue(),
    )
    .unwrap()
    .gestures;
    assert!(codec::export(&lua).is_err());
    let mut native = codec::inspect("gesture = 3,left,workspace\n")
        .unwrap()
        .draft;
    native.declarations.extend(lua.declarations);
    let outcomes = native.registrations(BASELINE).unwrap();
    assert!(outcomes.iter().all(|o| o.result.is_ok()));
    native.declarations[0].action = GestureAction::Callbacks(Default::default());
    assert!(native.validate(BASELINE).is_err());
}
#[test]
fn native_gesture_private_markers_preserve_intentional_empty_targets() {
    use hyprland_editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        workspace::{SourceLanguage, Workspace},
        Draft,
    };
    let source = "gesture = 3,left,workspace\n";
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
        .with_gestures(codec::inspect(source).unwrap().draft);
    w.sources[0].language = SourceLanguage::Legacy;
    assert!(w.validate(&catalogue()).is_err());
    w.legacy_gesture_origin_sha256 = Some(hash);
    w.legacy_gesture_schema_sha256 = Some(codec::schema_hash());
    w.validate(&catalogue()).unwrap();
    w.gestures.declarations.clear();
    let b = w.to_json(&catalogue()).unwrap();
    let r = Workspace::from_json(std::str::from_utf8(&b).unwrap(), &catalogue()).unwrap();
    assert!(r.gestures.is_empty());
    assert!(r.legacy_gesture_origin_sha256.is_some());
    w.legacy_gesture_schema_sha256 = Some("0".repeat(64));
    assert!(w.validate(&catalogue()).is_err());
}
