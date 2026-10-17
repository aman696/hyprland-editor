use hyprland_editor_core::{
    lua::{replay::*, source_hash},
    AUDITED_MAIN, BASELINE,
};
use serde_json::{json, Value};

fn document(id: &str, source: &str) -> Document {
    Document {
        id: id.into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: format!("<root>/config/{id}").into_bytes(),
    }
}
fn program(source: &str) -> Program {
    Program {
        version: BASELINE.into(),
        profile: Profile::SyntheticEmit,
        root: "main.lua".into(),
        documents: vec![document("main.lua", source)],
        bindings: vec![],
        initial_package_path_unknown_tail: false,
        initial_package_path: Some(b"<root>/config/?.lua;<root>/config/?/init.lua".to_vec()),
        working_directory: Some(b"<root>".to_vec()),
    }
}
fn observed(value: &Observed) -> Value {
    match value {
        Observed::Nil => Value::Null,
        Observed::Bool(v) => json!(v),
        Observed::Int(v) => json!(v),
        Observed::Float(v) => json!(v),
        Observed::Bytes(v) => json!(String::from_utf8(v.clone()).expect("UTF-8 fixture")),
        Observed::Function(_) => json!("function"),
        Observed::Cycle(_) => json!("<cycle>"),
        Observed::Table(rows) => {
            if !rows.is_empty()
                && rows
                    .iter()
                    .enumerate()
                    .all(|(i, (k, _))| *k == Observed::Int(i as i64 + 1))
            {
                Value::Array(rows.iter().map(|(_, v)| observed(v)).collect())
            } else {
                Value::Object(
                    rows.iter()
                        .map(|(k, v)| {
                            (
                                match k {
                                    Observed::Bytes(v) => String::from_utf8(v.clone()).unwrap(),
                                    Observed::Int(v) => v.to_string(),
                                    _ => "<non-string-key>".into(),
                                },
                                observed(v),
                            )
                        })
                        .collect(),
                )
            }
        }
    }
}

fn forwarded_literal<'a>(report: &'a Report, source: &ValueSource) -> &'a ExpressionSource {
    let mut id = source.expression.expect("Missing expression evidence");
    for _ in 0..128 {
        let node = &report.expression_sources[id as usize];
        if node.kind == ExpressionKind::Literal {
            return node;
        }
        id = node.selected.expect("No selected literal in this fixture");
    }
    panic!("Expression evidence cycle")
}

#[test]
fn value_sources_keep_equal_literals_separate_and_follow_alias_mutation_and_closure_returns() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program("local part=require('part');local alias=part;local function pass(x)return x end;emit({pass(alias.left),alias.right});alias.left=9;emit({alias.left,part.right});emit({part.right+1})");
        p.version = version.into();
        p.documents
            .push(document("part.lua", "return {left=2,right=2}"));
        let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
            panic!("No requested module")
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part.lua".into(),
                loader_data: vec![],
            },
        });
        let report = replay(&p).unwrap();
        assert!(report.halt.is_none(), "{:?}", report.halt);
        let first = &report.events[0].argument_sources[0].rows;
        let left = forwarded_literal(&report, &first[0]);
        let right = forwarded_literal(&report, &first[1]);
        assert_eq!((left.document, right.document), (1, 1));
        assert_ne!(
            left.span, right.span,
            "Equal primitive values must not merge distinct origins"
        );
        assert_eq!(&p.documents[1].source[left.span.clone()], "2");
        let second = &report.events[1].argument_sources[0].rows;
        let changed = forwarded_literal(&report, &second[0]);
        assert_eq!(changed.document, 0);
        assert_eq!(&p.documents[0].source[changed.span.clone()], "9");
        assert_eq!(forwarded_literal(&report, &second[1]), right);
        assert_eq!(observed(&report.events[0].arguments[0]), json!([2, 2]));
        assert_eq!(observed(&report.events[1].arguments[0]), json!([9, 2]));
        let derived_id = report.events[2].argument_sources[0].rows[0]
            .expression
            .unwrap() as usize;
        let derived = &report.expression_sources[derived_id];
        assert_eq!(derived.kind, ExpressionKind::Operation);
        assert!(derived.selected.is_none());
        assert_eq!(derived.evaluated.len(), 2);
        for (id, source) in report.expression_sources.iter().enumerate() {
            let doc = &p.documents[source.document];
            assert!(doc.source.get(source.span.clone()).is_some());
            assert_eq!(
                source.line,
                doc.source[..source.span.start]
                    .bytes()
                    .filter(|&b| b == b'\n')
                    .count()
                    + 1
            );
            assert!(
                source
                    .evaluated
                    .iter()
                    .chain(source.selected.iter())
                    .all(|&prior| (prior as usize) < id),
                "Evidence must be an acyclic backward graph"
            );
        }
        let decoded: Report =
            serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(decoded.expression_sources, report.expression_sources);
    }
}

#[test]
fn cached_tables_and_multireturn_preserve_source_selection_without_changing_identity() {
    let mut p=program("local a=require('part');local b=require('part');local function choose(x,y)return y,x end;emit({choose(a.right,a.left)});emit(a==b);a.left=7;emit(b)");
    p.documents
        .push(document("part.lua", "return {left=2,right=3}"));
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!("No module request")
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part.lua".into(),
            loader_data: vec![],
        },
    });
    let report = replay(&p).unwrap();
    assert!(report.halt.is_none());
    assert_eq!(observed(&report.events[0].arguments[0]), json!([2, 3]));
    assert_eq!(observed(&report.events[1].arguments[0]), json!(true));
    let rows = &report.events[0].argument_sources[0].rows;
    assert_eq!(
        &p.documents[1].source[forwarded_literal(&report, &rows[0]).span.clone()],
        "2"
    );
    assert_eq!(
        &p.documents[1].source[forwarded_literal(&report, &rows[1]).span.clone()],
        "3"
    );
    assert_eq!(report.events[2].argument_sources[0].rows.len(), 2);
    assert!(matches!(report.loads[1].outcome, LoadOutcome::Cached));
}

#[test]
fn expression_evidence_budget_cannot_be_swallowed_by_pcall_and_cycles_stay_bounded() {
    let report = replay(&program(
        "pcall(function()local x=0;for i=1,10000 do x=x+1 end end);emit('after')",
    ))
    .unwrap();
    assert!(
        matches!(report.halt,Some(Halt::Budget{ref reason}) if reason.contains("provenance")||reason.contains("source evidence"))
    );
    assert!(report.events.is_empty());
    assert!(report.expression_sources.len() <= 8192);
    assert!(serde_json::to_vec(&report).unwrap().len() < 2 * 1024 * 1024);
    let report = replay(&program("local t={};t.self=t;emit(t)")).unwrap();
    assert!(report.halt.is_none());
    let source = &report.events[0].argument_sources[0];
    assert_eq!(source.rows.len(), 1);
    assert!(source.rows[0].rows.is_empty());
}

#[test]
fn unknown_inherited_path_provenance_survives_aliases_and_literal_replacement() {
    let mut p=program("local saved=package.path; require('first'); package.path='/vendor/?.lua'; require('second'); package.path=saved; require('third'); emit(type(package.path))");
    p.initial_package_path_unknown_tail = true;
    let prefix = p.initial_package_path.clone().unwrap();
    p.documents.push(document("part.lua", "return 2"));
    for (name, path, unknown) in [
        ("first", prefix.clone(), true),
        ("second", b"/vendor/?.lua".to_vec(), false),
        ("third", prefix, true),
    ] {
        let report = replay(&p).unwrap();
        let Some(Halt::NeedsResolution { request }) = report.halt else {
            panic!("Expected resolution");
        };
        assert_eq!(request.module, name.as_bytes());
        assert_eq!(request.package_path, Some(path));
        assert_eq!(request.package_path_unknown_tail, unknown);
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part.lua".into(),
                loader_data: vec![],
            },
        });
    }
    let report = replay(&p).unwrap();
    assert!(report.halt.is_none());
    assert_eq!(observed(&report.events[0].arguments[0]), json!("string"));
    let roundtrip: Program = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(roundtrip.bindings[0].request, p.bindings[0].request);
    let mut stale = roundtrip;
    stale.bindings[0].request.package_path_unknown_tail = false;
    assert!(matches!(
        replay(&stale).unwrap().halt,
        Some(Halt::NeedsResolution { .. })
    ));
}

#[test]
fn known_path_prepends_are_preserved_without_fabricating_the_tail() {
    let mut p = program("package.path='/before/?.lua;'..package.path; require('first')");
    p.initial_package_path_unknown_tail = true;
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!()
    };
    assert!(request.package_path_unknown_tail);
    assert_eq!(
        request.package_path.unwrap(),
        [
            b"/before/?.lua;".as_slice(),
            p.initial_package_path.as_ref().unwrap()
        ]
        .concat()
    );
    p.initial_package_path = None;
    assert!(replay(&p).is_err());
}

#[test]
fn uncertain_path_operations_are_not_lua_errors_or_known_observations() {
    for source in [
        "pcall(function()emit(package.path==package.path)end);emit('after')",
        "pcall(function()local t={};t[package.path]=1 end);emit('after')",
        "pcall(error,package.path);emit('after')",
        "pcall(function()package.path=package.path..';/after/?.lua' end);emit('after')",
        "package.path='bad\\0;'..package.path;require('first');emit('after')",
    ] {
        let mut p = program(source);
        p.initial_package_path_unknown_tail = true;
        let report = replay(&p).unwrap();
        assert!(
            matches!(report.halt, Some(Halt::Unresolved { .. })),
            "{source}: {:?}",
            report.halt
        );
        assert!(report.events.is_empty(), "{source}");
    }
}

#[test]
fn ordered_reference_observations() {
    let mut fixtures: Value =
        serde_json::from_str(include_str!("../../../data/ordered-module-fixtures.json")).unwrap();
    assert_eq!(fixtures["format"], "hye-ordered-module-fixtures-1");
    assert_eq!(fixtures["cases"].as_array().unwrap().len(), 96);
    assert_eq!(
        fixtures["audit_manifest_sha256"],
        source_hash(include_str!("../../../data/source-graph-semantics.json"))
    );
    let matrix: Value =
        serde_json::from_str(include_str!("../../../data/lua-profiles.json")).unwrap();
    let profile = compiler_profile();
    assert_eq!(
        matrix["canonical_fixtures_sha256"],
        source_hash(include_str!("../../../data/ordered-module-fixtures.json"))
    );
    let row = matrix["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["release"] == profile.release)
        .expect("Unverified linked Lua profile");
    for change in row["ordered_overrides"].as_array().unwrap() {
        let case = fixtures["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["case"] == change["case"] && case["version"] == change["version"])
            .unwrap();
        for (field, value) in change["fields"].as_object().unwrap() {
            case[field] = value.clone();
        }
    }
    let mut failures = vec![];
    for case in fixtures["cases"].as_array().unwrap() {
        let mut p = program(case["source"].as_str().unwrap());
        p.version = if case["version"] == "baseline" {
            BASELINE
        } else {
            AUDITED_MAIN
        }
        .into();
        for (id, source) in fixtures["module_files"].as_object().unwrap() {
            p.documents.push(document(id, source.as_str().unwrap()));
        }
        let report = loop {
            let r = replay(&p).unwrap();
            let Some(Halt::NeedsResolution { request }) = &r.halt else {
                break r;
            };
            assert!(p.bindings.len() < 100, "Fixture resolution loop");
            // This resolver owns the complete private fixture namespace. It does
            // not implement or certify production path discovery/authorization.
            let module = String::from_utf8(request.module.clone()).unwrap();
            let id = if module.starts_with("<root>/config/") {
                module.trim_start_matches("<root>/config/").to_owned()
            } else {
                format!("{}.lua", module.replace('.', "/"))
            };
            let resolution = if p.documents.iter().any(|d| d.id == id) {
                Resolution::Loaded {
                    document: id.clone(),
                    loader_data: format!("<root>/config/{id}").into_bytes(),
                }
            } else {
                Resolution::Missing
            };
            p.bindings.push(Binding {
                request: request.clone(),
                resolution,
            });
        };
        assert!(!report.runtime_verified);
        assert_eq!(report.compiler_profile, profile);
        let trace: Vec<Value> = report
            .events
            .iter()
            .filter_map(|e| {
                let first = e.arguments.first().unwrap_or(&Observed::Nil);
                if e.api == "hl.config" {
                    Some(json!({"__api":"hl.config","value":observed(first)}))
                } else if *first == Observed::Nil {
                    None
                } else {
                    Some(observed(first))
                }
            })
            .collect();
        let ok = report.halt.is_none();
        if json!(trace) != case["trace"]
            || json!(ok) != case["ok"]
            || json!(report.logged_module_errors) != case["error_count"]
            || matches!(
                report.halt,
                Some(Halt::Unresolved { .. } | Halt::Budget { .. })
            )
        {
            failures.push(format!(
                "{} {}: expected {} / ok={} / errors={}, got {} / halt={:?} / errors={}",
                case["version"],
                case["case"],
                case["trace"],
                case["ok"],
                case["error_count"],
                json!(trace),
                report.halt,
                report.logged_module_errors
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unresolved_native_effect_is_never_caught_as_a_lua_error_or_skipped() {
    let p = program("emit('before');pcall(function()os.execute('never')end);emit('after')");
    let r = replay(&p).unwrap();
    assert_eq!(r.events.len(), 1);
    assert!(matches!(r.halt, Some(Halt::Unresolved { .. })));
    assert_eq!(r.logged_module_errors, 0);
}

#[test]
fn module_resolution_is_bound_to_exact_source_and_call_context() {
    let mut p = program("emit('before');require('missing');emit('after')");
    let r = replay(&p).unwrap();
    let Some(Halt::NeedsResolution { request }) = r.halt else {
        panic!("Expected resolution request")
    };
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.logged_module_errors, 0);
    p.bindings.push(Binding {
        request: request.clone(),
        resolution: Resolution::Missing,
    });
    assert!(matches!(
        replay(&p).unwrap().halt,
        Some(Halt::Unresolved { .. })
    ));
    p.bindings[0].request.package_path = Some(b"different/?.lua".to_vec());
    assert!(matches!(
        replay(&p).unwrap().halt,
        Some(Halt::NeedsResolution { .. })
    ));
    p.bindings[0].request = request;
    p.bindings[0].request.sha256 = "0".repeat(64);
    assert!(replay(&p).is_err());
}

#[test]
fn syntax_and_const_errors_precede_all_observations() {
    for source in [
        "emit('never');local x <const> = 1;x=2",
        "emit('never');return ...;local x",
        "emit('never');local function f()return ... end",
    ] {
        let r = replay(&program(source)).unwrap();
        assert!(r.events.is_empty());
        assert!(
            matches!(r.halt, Some(Halt::LuaError { .. })),
            "{source}: {:?}",
            r.halt
        );
    }
}

#[test]
fn cyclic_and_shared_tables_have_bounded_observations_without_losing_identity() {
    let r = replay(&program(
        "local t={};t.self=t;emit(t);local a={v=1};emit({a,a});a.v=2;emit(a)",
    ))
    .unwrap();
    assert!(r.halt.is_none());
    assert!(
        matches!(&r.events[0].arguments[0], Observed::Table(rows) if matches!(rows[0].1, Observed::Cycle(_)))
    );
    assert_eq!(
        observed(&r.events[1].arguments[0]),
        json!([{"v":1},{"v":1}])
    );
    assert_eq!(observed(&r.events[2].arguments[0]), json!({"v":2}));
}

#[test]
fn shared_table_expansion_and_repeated_output_respect_the_serialized_envelope() {
    for source in [
        "local t={v='leaf'};for i=1,24 do t={t,t}end;emit(t)",
        "local t={};for i=1,200 do t[i]='012345678901234567890123456789' end;for i=1,1000 do emit(t)end",
        "local t={};for i=1,20 do t[i]=i end;while true do emit(t)end",
    ] {
        let r = replay(&program(source)).unwrap();
        assert!(matches!(r.halt, Some(Halt::Budget {..})), "{:?}", r.halt);
        assert!(serde_json::to_vec(&r).unwrap().len() < 4 * 1024 * 1024);
    }
    let mut p = program(
        "local f=function()end;local t={};for i=1,1000 do t[i]=f end;for i=1,20 do emit(t)end",
    );
    p.documents[0].id = "\u{1}".repeat(64);
    p.root = p.documents[0].id.clone();
    let r = replay(&p).unwrap();
    assert!(matches!(r.halt, Some(Halt::Budget { .. })));
    assert!(serde_json::to_vec(&r).unwrap().len() < 4 * 1024 * 1024);
}

#[test]
fn unbounded_control_flow_stops_and_cannot_be_swallowed_by_pcall() {
    for source in [
        "pcall(function()while true do end end);emit('never')",
        "local function f()return f()end;pcall(f);emit('never')",
    ] {
        let r = replay(&program(source)).unwrap();
        assert!(r.events.is_empty());
        assert!(matches!(r.halt, Some(Halt::Budget { .. })), "{:?}", r.halt);
    }
}

#[test]
fn bundle_identity_and_context_limits_are_checked_before_replay() {
    let mut p = program("emit(1)");
    p.documents[0].source = "emit(2)".into();
    assert!(replay(&p).is_err());
    p.documents[0].sha256 = source_hash(&p.documents[0].source);
    p.working_directory = Some(vec![b'x'; 4097]);
    assert!(replay(&p).is_err());
    p.working_directory = Some(b"/tmp\0other".to_vec());
    assert!(replay(&p).is_err());
    p.working_directory = None;
    p.documents.push(p.documents[0].clone());
    assert!(replay(&p).is_err());
}

#[test]
fn custom_searchers_and_unknown_library_state_stop_before_later_observations() {
    for source in [
        "package.searchers={};require('a');emit('never')",
        "emit(os);emit('never')",
    ] {
        let r = replay(&program(source)).unwrap();
        assert!(r.events.is_empty());
        assert!(
            matches!(r.halt, Some(Halt::Unresolved { .. })),
            "{:?}",
            r.halt
        );
    }
}

#[test]
fn loader_edges_and_api_calls_share_actual_order_and_document_provenance() {
    let mut p = program("hl.config{root=1};local a=require('a');hl.config{last=a.n};require('b')");
    p.documents.push(document(
        "a.lua",
        "hl.config{a=1};local b=require('b');return b",
    ));
    p.documents
        .push(document("b.lua", "hl.config{b=2};return {n=7}"));
    let r = loop {
        let r = replay(&p).unwrap();
        let Some(Halt::NeedsResolution { request }) = &r.halt else {
            break r;
        };
        assert!(p.bindings.len() < 3);
        let id = format!("{}.lua", String::from_utf8(request.module.clone()).unwrap());
        p.bindings.push(Binding {
            request: request.clone(),
            resolution: Resolution::Loaded {
                document: id,
                loader_data: vec![],
            },
        });
    };
    assert!(r.halt.is_none());
    assert_eq!(
        r.events
            .iter()
            .map(|e| (e.order, e.document.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "main.lua"), (2, "a.lua"), (4, "b.lua"), (5, "main.lua")]
    );
    assert_eq!(
        r.loads.iter().map(|e| e.order).collect::<Vec<_>>(),
        vec![1, 3, 6]
    );
    assert!(
        matches!(&r.loads[0].outcome,LoadOutcome::SuppliedSnapshot{document,..} if document=="a.lua")
    );
    assert!(matches!(r.loads[2].outcome, LoadOutcome::Cached));
    for edge in r.loads {
        let doc = p
            .documents
            .iter()
            .find(|d| d.id == edge.request.document)
            .unwrap();
        assert_eq!(doc.sha256, edge.request.sha256);
        assert!(doc.source[edge.request.span].starts_with("require("));
    }
}

#[test]
fn loader_path_repetition_and_uncertain_error_objects_remain_bounded_and_explicit() {
    let source = format!("package.path='{}';package.preload.x=function()return {{}} end;for i=1,500 do require('x')end", "x".repeat(25_000));
    let r = replay(&program(&source)).unwrap();
    assert!(matches!(r.halt, Some(Halt::Budget { .. })));
    assert!(serde_json::to_vec(&r).unwrap().len() < 4 * 1024 * 1024);
    for source in ["local function f()local x=nil;return x.foo end;package.preload.x=f;require('x');emit('never')", "local x=nil;x()"] {
        let r = replay(&program(source)).unwrap();
        assert!(r.events.is_empty());
        assert!(matches!(r.halt,Some(Halt::Unresolved{..})),"{:?}",r.halt);
        assert_eq!(r.logged_module_errors,0);
    }
    let mut p = program("if absent_native_global then emit('never') end");
    p.profile = Profile::HyprlandLua54;
    assert!(matches!(
        replay(&p).unwrap().halt,
        Some(Halt::Unresolved { .. })
    ));
}

#[test]
fn local_function_binding_uses_syntax_instead_of_whitespace_spelling() {
    for separator in [" ", "\t", "\n", " -- comment\n"] {
        let source =
            format!("local f=3;do local{separator}function f()return 9 end;emit(f())end;emit(f)");
        let r = replay(&program(&source)).unwrap();
        assert!(r.halt.is_none(), "{:?}", r.halt);
        assert_eq!(
            r.events
                .iter()
                .map(|e| observed(&e.arguments[0]))
                .collect::<Vec<_>>(),
            vec![json!(9), json!(3)]
        );
    }
}

#[test]
fn lua_compiler_allocation_and_diagnostic_budgets_stop_before_tree_evaluation() {
    use std::fmt::Write;
    let mut source = "return {".to_owned();
    for i in 0..90_000 {
        write!(&mut source, "k{i}=1,").unwrap();
    }
    source.push('}');
    assert!(source.len() < 1024 * 1024);
    let r = replay(&program(&source)).unwrap();
    assert!(r.events.is_empty());
    assert!(
        matches!(r.halt,Some(Halt::Budget {ref reason}) if reason.contains("compilation")),
        "{:?}",
        r.halt
    );
    let name = "a".repeat(5000);
    let r = replay(&program(&format!("local {name} <const> = 1;{name}=2"))).unwrap();
    assert!(r.events.is_empty());
    assert!(
        matches!(r.halt,Some(Halt::Budget {ref reason}) if reason.contains("diagnostic")),
        "{:?}",
        r.halt
    );
}

#[test]
fn unsupported_close_and_loadfile_mode_semantics_do_not_produce_later_calls() {
    for source in [
        "local value <close> = 1;emit('never')",
        "loadfile('/fixture/file',false);emit('never')",
        "loadfile('/fixture/file',1);emit('never')",
    ] {
        let r = replay(&program(source)).unwrap();
        assert!(r.events.is_empty());
        assert!(
            matches!(r.halt, Some(Halt::Unresolved { .. })),
            "{source}: {:?}",
            r.halt
        );
    }
}
