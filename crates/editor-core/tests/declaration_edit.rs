use hyprland_editor_core::{
    catalogue,
    lua::{
        self,
        replay::{self, declaration_edit, Binding, Document, Profile, Program, Resolution},
    },
    workspace::Workspace,
    AUDITED_MAIN, BASELINE,
};
fn document(id: &str, source: &str) -> Document {
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
        documents: vec![document("main.lua", source)],
        bindings: vec![],
        initial_package_path: Some(b"/config/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: Some(b"/config".to_vec()),
    }
}
fn target(source: &str, version: &str) -> Workspace {
    let r = lua::parse(source, version, &catalogue()).unwrap();
    assert!(
        !r.opaque_effects && r.diagnostics.is_empty(),
        "{:?}",
        r.diagnostics
    );
    Workspace::new(r.draft)
        .with_devices(r.devices)
        .with_bindings(r.bindings)
        .with_workspace_rules(r.workspace_rules)
        .with_rules(r.rules)
        .with_animations(r.animations)
        .with_gestures(r.gestures)
}
#[test]
fn graphical_forms_update_each_supported_family_in_a_captured_module() {
    for version in [BASELINE, AUDITED_MAIN] {
        for (old,new) in [
   ("hl.device{-- retained comment\n name='mouse',sensitivity=0.1}","hl.device{name='mouse',sensitivity=0.3}"),
   ("hl.bind('Q',hl.dsp.no_op())","hl.bind('W',hl.dsp.no_op())"),
   ("hl.workspace_rule{workspace='1'}","hl.workspace_rule{workspace='2'}"),
   ("hl.window_rule{match={class='term'},float=true}","hl.window_rule{match={class='term'},float=false}"),
   ("hl.layer_rule{match={namespace='panel'},blur=true}","hl.layer_rule{match={namespace='panel'},blur=false}"),
   ("hl.curve('ease',{type='bezier',points={{0.1,0.2},{0.8,0.9}}})","hl.curve('ease',{type='bezier',points={{0.2,0.3},{0.7,0.8}}})"),
   ("hl.animation{leaf='windows',enabled=true,speed=1,bezier='linear'}","hl.animation{leaf='windows',enabled=false,speed=1,bezier='linear'}"),
   ("hl.gesture{fingers=3,direction='left',action={start=function()hl.dispatch(hl.dsp.no_op())end,finish=function()end}}","hl.gesture{fingers=4,direction='left',action={start=function()hl.dispatch(hl.dsp.no_op())end,finish=function()end}}"),
  ] {
   let mut p=program("local value=require('part');hl.config{general={border_size=value}}",version);
   p.documents.push(document("part.lua",&format!("-- module header\n{old};return 2")));
   let request=match replay::replay(&p).unwrap().halt.unwrap(){replay::Halt::NeedsResolution{request}=>request,other=>panic!("{other:?}")};
   p.bindings.push(Binding{request,resolution:Resolution::Loaded{document:"part.lua".into(),loader_data:vec![]}});
   let observed=replay::replay(&p).unwrap(); assert!(observed.halt.is_none());
   let order=observed.events[0].order;
   let inspected=declaration_edit::inspect(&p,order,&catalogue()).unwrap_or_else(|e|panic!("{old}: {e}; {:?}",lua::parse(old,version,&catalogue()).unwrap().diagnostics)); assert!(!inspected.opaque_effects);
   let result=declaration_edit::compare(&p,order,&target(new,version),&catalogue()).unwrap_or_else(|e|panic!("{old}: {e}"));
   assert!(result.review.modeled_observations_match,"{old}: {:?}",result.review.differences);
   assert_eq!(result.review.expected_calls_matched,1);
   assert_eq!(result.review.revisions.len(),1);
   assert_eq!(result.review.revisions[0].document,"part.lua");
   assert!(result.review.revisions[0].content.starts_with("-- module header"));
   assert!(result.review.revisions[0].content.ends_with(";return 2"));
   if old.contains("retained comment"){assert!(result.review.revisions[0].content.contains("-- retained comment"));}
   assert_eq!(p.documents[1].source,format!("-- module header\n{old};return 2"));
  }
    }
}
#[test]
fn repeated_consumed_and_changed_family_inputs_cannot_open_partial_forms() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "for i=1,2 do hl.device{name='mouse'}end",
            "local rule=hl.workspace_rule{workspace='1'}",
        ] {
            let p = program(source, version);
            let report = replay::replay(&p).unwrap();
            let e = report.events.first().unwrap();
            assert!(
                declaration_edit::inspect(&p, e.order, &catalogue()).is_err(),
                "{source}"
            );
        }
        let p = program("hl.device{name='mouse',sensitivity=0.1}", version);
        assert!(declaration_edit::compare(
            &p,
            0,
            &target("hl.device{name='mouse',sensitivity=0.1}", version),
            &catalogue()
        )
        .is_err());
        assert!(declaration_edit::compare(
            &p,
            0,
            &target("hl.device{name='mouse'};hl.device{name='other'}", version),
            &catalogue()
        )
        .is_err());
    }
}

#[test]
fn binding_edits_preserve_enclosing_submap_and_rebase_later_captured_loads() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("hl.define_submap('demo',function()require('part')end);hl.config{general={border_size=2}}",version);
        p.documents.push(document(
            "part.lua",
            "hl.bind('Q',hl.dsp.no_op());local value=require('later');return value",
        ));
        p.documents.push(document(
            "later.lua",
            "hl.device{name='untouched'};return 2",
        ));
        for target in ["part.lua", "later.lua"] {
            let request = match replay::replay(&p).unwrap().halt.unwrap() {
                replay::Halt::NeedsResolution { request } => request,
                other => panic!("{other:?}"),
            };
            p.bindings.push(Binding {
                request,
                resolution: Resolution::Loaded {
                    document: target.into(),
                    loader_data: vec![],
                },
            });
        }
        let before = replay::replay(&p).unwrap();
        assert!(before.halt.is_none());
        let selected = before.events.iter().find(|e| e.api == "hl.bind").unwrap();
        let result = declaration_edit::compare(
            &p,
            selected.order,
            &target("hl.bind('W',hl.dsp.exec_cmd('echo example'))", version),
            &catalogue(),
        )
        .unwrap();
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        assert_eq!(result.review.expected_calls_matched, 1);
        assert!(result.review.before_model_complete && result.review.after_model_complete);
        assert!(!result.review.runtime_verified);
        assert!(result.review.revisions[0]
            .content
            .contains("require('later')"));
    }
}

#[test]
fn captured_computed_values_populate_forms_without_changing_shared_locals() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p=program("local name='mouse';local gain=0.2;hl.device{name=name,sensitivity=gain};hl.config{input={sensitivity=gain}}",version);
        let form = declaration_edit::inspect(&p, 0, &catalogue()).unwrap();
        assert_eq!(form.devices[0].name, "mouse");
        let result = declaration_edit::compare(
            &p,
            0,
            &target("hl.device{name='new mouse',sensitivity=0.4}", version),
            &catalogue(),
        )
        .unwrap();
        assert!(
            result.review.modeled_observations_match,
            "{:?}",
            result.review.differences
        );
        assert!(result.review.revisions[0]
            .content
            .starts_with("local name='mouse';local gain=0.2;"));
        assert!(result.review.revisions[0]
            .content
            .ends_with("hl.config{input={sensitivity=gain}}"));
    }
}
