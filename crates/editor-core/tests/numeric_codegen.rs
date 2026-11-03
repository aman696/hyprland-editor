use hyprland_editor_core::{
    lua::{self, replay::*},
    AUDITED_MAIN, BASELINE,
};

#[test]
fn ordered_numeric_values_match_the_observed_compiler_not_the_library_version_label() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/lua-numeric-fixtures.json")).unwrap();
    let compiler = compiler_profile();
    let profile = compiler.numeric_codegen.as_ref().expect("compiler probe");
    let expected = reference["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["probe_sha256"] == profile.probe_sha256)
        .expect("reviewed current compiler");
    for version in [BASELINE, AUDITED_MAIN] {
        for (index, source) in reference["sources"].as_array().unwrap().iter().enumerate() {
            let source = source.as_str().unwrap();
            let p = Program {
                version: version.into(),
                profile: Profile::SyntheticEmit,
                root: "root".into(),
                documents: vec![Document {
                    id: "root".into(),
                    source: source.into(),
                    sha256: lua::source_hash(source),
                    load_name: b"/numeric.lua".to_vec(),
                }],
                bindings: vec![],
                initial_package_path: Some(vec![]),
                initial_package_path_unknown_tail: false,
                working_directory: None,
            };
            let report = replay(&p).unwrap();
            assert!(report.halt.is_none(), "{source}: {:?}", report.halt);
            let [Observed::Float(number)] = report.returns.as_slice() else {
                panic!("numeric result")
            };
            assert_eq!(
                format!("{:016x}", number.to_bits()),
                expected["observations"][index],
                "{source}"
            );
            assert_eq!(report.compiler_profile, compiler);
            assert!(!report.runtime_verified);
        }
    }
}

#[test]
fn source_comparison_retains_compiler_zero_sign_and_rejects_changed_compiler_claims() {
    for version in [BASELINE, AUDITED_MAIN] {
        let source = "local nz=-0.0;local z=0;return nz-0";
        let p = Program {
            version: version.into(),
            profile: Profile::SyntheticEmit,
            root: "root".into(),
            documents: vec![Document {
                id: "root".into(),
                source: source.into(),
                sha256: lua::source_hash(source),
                load_name: b"/numeric.lua".to_vec(),
            }],
            bindings: vec![],
            initial_package_path: Some(vec![]),
            initial_package_path_unknown_tail: false,
            working_directory: None,
        };
        let end = source.len();
        let mut plan = edits::prepare(
            &p,
            vec![edits::SourceEdit {
                document: "root".into(),
                base_sha256: p.documents[0].sha256.clone(),
                span: end - 1..end,
                old_bytes_sha256: lua::source_hash("0"),
                replacement: "z".into(),
                semantic_targets: vec!["numeric-return".into()],
            }],
            vec![],
        )
        .unwrap();
        let review = edits::review(&p, &plan).unwrap();
        let rewrites = compiler_profile().numeric_codegen.unwrap().zero_subtraction
            == Some(ZeroSubtraction::RewritesConstantIntegerZero);
        assert_eq!(review.modeled_observations_match, !rewrites);
        assert_eq!(
            review
                .differences
                .iter()
                .any(|d| d.kind == edits::DifferenceKind::ReturnValues),
            rewrites
        );
        assert!(review.before_model_complete && review.after_model_complete);
        assert!(!review.runtime_verified && !review.ownership_verified);
        plan.compiler.numeric_codegen.as_mut().unwrap().probe_sha256 = "0".repeat(64);
        assert!(edits::review(&p, &plan).is_err());
        plan.compiler = compiler_profile();
        plan.compiler.numeric_codegen = None;
        assert!(edits::review(&p, &plan).is_err());
    }
}
