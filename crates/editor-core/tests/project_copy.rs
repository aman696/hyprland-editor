use hyprland_editor_core::{
    catalogue,
    lua::{
        self,
        replay::{
            self,
            project_copy::{self, Mode},
            Binding, Document, Profile, Program, Resolution,
        },
    },
    workspace::Workspace,
    AUDITED_MAIN, BASELINE,
};
fn doc(id: &str, source: &str) -> Document {
    Document {
        id: id.into(),
        source: source.into(),
        sha256: lua::source_hash(source),
        load_name: format!("/config/{id}").into_bytes(),
    }
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: if cfg!(feature = "native55") {
            Profile::HyprlandLua55
        } else {
            Profile::HyprlandLua54
        },
        root: "main.lua".into(),
        documents: vec![doc("main.lua", source)],
        bindings: vec![],
        initial_package_path: Some(b"/config/?.lua".into()),
        initial_package_path_unknown_tail: false,
        working_directory: Some(b"/config".into()),
    }
}
fn resolve(p: &mut Program, id: &str) {
    let request = match replay::replay(p).unwrap().halt.unwrap() {
        replay::Halt::NeedsResolution { request } => request,
        other => panic!("{other:?}"),
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: id.into(),
            loader_data: vec![],
        },
    });
}
fn target(source: &str, version: &str) -> Workspace {
    let r = lua::parse(source, version, &catalogue()).unwrap();
    assert!(
        r.diagnostics.is_empty() && !r.opaque_effects,
        "{:?}",
        r.diagnostics
    );
    Workspace::new(r.draft)
        .with_monitors(r.monitors)
        .with_devices(r.devices)
        .with_bindings(r.bindings)
        .with_workspace_rules(r.workspace_rules)
        .with_rules(r.rules)
        .with_animations(r.animations)
        .with_gestures(r.gestures)
        .with_session(r.session)
}
const OLD:&str="hl.monitor{output='DP-1'};hl.device{name='old'};hl.define_submap('oldmap',function()hl.bind('Q',hl.dsp.no_op())end);hl.workspace_rule{workspace='1'};hl.window_rule{match={class='old'},float=true};hl.layer_rule{match={namespace='old'},blur=true};hl.curve('oldcurve',{type='bezier',points={{0.1,0.2},{0.8,0.9}}});hl.animation{leaf='windows',enabled=false};hl.gesture{fingers=3,direction='left',action='workspace'};hl.env('GTK_THEME','Old');hl.on('hyprland.start',function()hl.exec_cmd('old application')end);hl.permission{binary='^/usr/bin/old$',type='screencopy',mode='ask'}";
const NEW:&str="hl.config{general={border_size=7}};hl.monitor{output='DP-2'};hl.device{name='new'};hl.define_submap('newmap',function()hl.bind('W',hl.dsp.no_op())end);hl.workspace_rule{workspace='2'};hl.window_rule{match={class='new'},float=false};hl.layer_rule{match={namespace='new'},blur=false};hl.curve('newcurve',{type='bezier',points={{0.2,0.3},{0.7,0.8}}});hl.animation{leaf='windows',enabled=true,speed=2,bezier='newcurve'};hl.gesture{fingers=4,direction='right',action='workspace'};hl.env('GTK_THEME','New');hl.on('hyprland.shutdown',function()hl.exec_cmd('new application')end);hl.permission{binary='^/usr/bin/new$',type='keyboard',mode='deny'}";
#[test]
fn all_nine_draft_groups_combine_with_multifile_calls_and_module_returns() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("-- root café\r\nhl.config{general={border_size=2}};local value=require('part');return value",version);
        p.documents.push(doc(
            "part.lua",
            &format!("-- module header\n{OLD};-- retained end\nreturn 9"),
        ));
        resolve(&mut p, "part.lua");
        let original = serde_json::to_vec(&p).unwrap();
        let copy = project_copy::prepare(
            &p,
            &target(NEW, version),
            [Mode::ReplaceReached; 9],
            &catalogue(),
        )
        .unwrap();
        assert_eq!(copy.files.len(), 2);
        assert!(copy.files.iter().all(|f| f.changed));
        assert!(copy.before_complete && copy.after_complete);
        assert_eq!(copy.removed_sites, 14);
        assert_eq!(copy.inserted_sites, 14);
        let root = &copy.files[0].content;
        let part = &copy.files[1].content;
        assert!(root.starts_with("-- root café\r\n"));
        assert!(root.ends_with("return value"));
        assert!(part.starts_with("-- module header\n"));
        assert!(part.ends_with("-- retained end\nreturn 9"));
        for old in ["oldmap", "oldcurve", "name='old'"] {
            assert!(!part.contains(old));
        }
        for new in ["DP-2", "newmap", "newcurve"] {
            assert!(root.contains(new));
        }
        let mut proposed = p.clone();
        proposed.bindings.clear();
        for doc in &mut proposed.documents {
            let file = copy.files.iter().find(|f| f.document == doc.id).unwrap();
            doc.source = file.content.clone();
            doc.sha256 = file.sha256.clone();
        }
        resolve(&mut proposed, "part.lua");
        let after = replay::replay(&proposed).unwrap();
        assert!(after.halt.is_none());
        assert_eq!(after.events.len(), 14);
        assert_eq!(after.loads.len(), 1);
        assert_eq!(
            serde_json::to_vec(&p).unwrap(),
            original,
            "review must not mutate its inputs"
        );
    }
}
#[test]
fn keep_retains_private_bytes_and_append_does_not_remove_source_calls() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program("require('part');-- root tail", version);
        p.documents.push(doc(
            "part.lua",
            "hl.device{name='private edited mouse'};return 4",
        ));
        resolve(&mut p, "part.lua");
        let target = target(NEW, version);
        let kept = project_copy::prepare(&p, &target, [Mode::Keep; 9], &catalogue()).unwrap();
        assert!(kept.files.iter().all(|f| !f.changed));
        assert_eq!(kept.files[1].content, p.documents[1].source);
        let mut modes = [Mode::Keep; 9];
        modes[2] = Mode::Append;
        let appended = project_copy::prepare(&p, &target, modes, &catalogue()).unwrap();
        assert_eq!(appended.removed_sites, 0);
        assert_eq!(appended.inserted_sites, 1);
        assert_eq!(appended.files[1].content, p.documents[1].source);
        assert!(appended.files[0]
            .content
            .starts_with(&p.documents[0].source));
        assert!(appended.findings.is_empty(), "{:?}", appended.findings);
    }
}
#[test]
fn replace_empty_is_explicit_and_adjacent_replacement_insertion_is_valid() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program("hl.device{-- keep this comment\nname='old'}", version);
        let mut modes = [Mode::Keep; 9];
        modes[2] = Mode::ReplaceReached;
        let removed = project_copy::prepare(&p, &target("", version), modes, &catalogue()).unwrap();
        assert!(removed.files[0].content.contains("-- keep this comment"));
        assert!(!removed.files[0].content.contains("hl.device"));
        assert_eq!(removed.removed_sites, 1);
        assert_eq!(removed.inserted_sites, 0);
        let changed = project_copy::prepare(
            &p,
            &target("hl.device{name='new'}", version),
            modes,
            &catalogue(),
        )
        .unwrap();
        assert!(changed.findings.is_empty(), "{:?}", changed.findings);
        assert!(changed.files[0].content.contains("-- keep this comment"));
        assert!(changed.files[0].content.contains("new"));
    }
}
#[test]
fn replacement_refuses_partial_consumed_and_cross_group_or_loader_containers() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut modes = [Mode::Keep; 9];
        modes[3] = Mode::ReplaceReached;
        let p=program("hl.define_submap('demo',function()hl.device{name='keep'};hl.bind('Q',hl.dsp.no_op())end)",version);
        assert!(
            project_copy::prepare(&p, &target("", version), modes, &catalogue())
                .unwrap_err()
                .contains("another group")
        );
        let mut p = program(
            "hl.define_submap('demo',function()require('part')end)",
            version,
        );
        p.documents
            .push(doc("part.lua", "hl.bind('Q',hl.dsp.no_op())"));
        resolve(&mut p, "part.lua");
        assert!(
            project_copy::prepare(&p, &target("", version), modes, &catalogue())
                .unwrap_err()
                .contains("module load")
        );
        modes[2] = Mode::ReplaceReached;
        let p = program("local consumed=hl.device{name='old'}", version);
        assert!(
            project_copy::prepare(&p, &target("", version), modes, &catalogue())
                .unwrap_err()
                .contains("consumed call")
        );
        let p = program("require('uncaptured')", version);
        assert!(
            project_copy::prepare(&p, &target("", version), modes, &catalogue())
                .unwrap_err()
                .contains("complete captured replay")
        );
        let p = program("local hl={};return 2", version);
        let mut append = [Mode::Keep; 9];
        append[2] = Mode::Append;
        assert!(project_copy::prepare(
            &p,
            &target("hl.device{name='new'}", version),
            append,
            &catalogue()
        )
        .is_err());
    }
}
