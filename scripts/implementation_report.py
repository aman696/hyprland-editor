#!/usr/bin/env python3
"""Keep the complete audit's obligations visible without turning test presence into certification."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
CONTRACTS = ROOT / "docs/hyprland-implementation-contracts.json"
BLUEPRINT = ROOT / "docs/engineering-blueprint.md"
PLAN = ROOT / "docs/complete-editor-plan.md"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def numbered_sections(path):
    text = path.read_text()
    headings = list(re.finditer(r"^## (\d+)\. (.+)$", text, re.M))
    return [
        {
            "id": f"blueprint-section-{heading[1]}",
            "requirement": heading[2],
            "source": str(path.relative_to(ROOT)),
            "source_line": text[:heading.start()].count("\n") + 1,
            "scope_sha256": digest(text[heading.start():headings[index + 1].start()
                if index + 1 < len(headings) else len(text)].encode()),
            "complete": False,
            "state": "not_proven",
        }
        for index, heading in enumerate(headings)
    ]


def table_rows(path, match):
    rows = []
    for line_number, line in enumerate(path.read_text().splitlines(), 1):
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if line.startswith("|") and re.fullmatch(match, cells[0]):
            if len(cells) != 3:
                raise ValueError(f"Unexpected requirement table at {path}:{line_number}")
            rows.append({"id": cells[0], "requirement": cells[1], "acceptance": cells[2],
                "source": str(path.relative_to(ROOT)), "source_line": line_number,
                "complete": False, "state": "not_proven"})
    return rows


def model_invariants():
    text = BLUEPRINT.read_text().split("Model invariants:\n", 1)[1].split("\nUndo stores", 1)[0]
    return [{"id": f"model-invariant-{number}", "requirement": requirement,
        "source": "docs/engineering-blueprint.md", "complete": False, "state": "not_proven"}
        for number, requirement in re.findall(r"^(\d+)\. (.+)$", text, re.M)]


def build():
    contracts = json.loads(CONTRACTS.read_bytes())
    features = contracts["features"]
    if Counter(row["kind"] for row in features) != Counter(contracts["feature_counts"]):
        raise ValueError("Contract kind counts differ from the pinned audit")
    inventory = ROOT / "docs/hyprland-feature-inventory.json"
    if digest(inventory.read_bytes()) != contracts["source_inventory_sha256"]:
        raise ValueError("Source inventory hash differs from the full contract manifest")
    compact = json.loads((ROOT / "data/features.json").read_bytes())
    if len({row["id"] for row in features}) != len(features):
        raise ValueError("Duplicate full contract IDs")
    if {row["id"] for row in features} != {row["id"] for row in compact}:
        raise ValueError("Application inventory differs from full audit contracts")
    security = table_rows(BLUEPRINT, r"S\d\d")
    security_ids = {row["id"] for row in security}
    obligations = []
    for feature in features:
        if not set(feature["tests_required"]).issubset(security_ids):
            raise ValueError(f"Unknown mandatory gate for {feature['id']}")
        obligations.append({
            "id": feature["id"], "kind": feature["kind"], "engine": feature["engine"],
            "versions": feature["version_membership"], "required_editor": feature["editor"],
            "required_codec": feature["codec"], "required_tests": feature["tests_required"],
            "coverage_state": "Specified", "complete": False,
            "foundation": "typed scalar form and initial-value smoke check; full per-field semantics remain unproven"
                if feature["kind"] == "scalar" else "typed monitor draft forms, static Lua projection, schema-pinned persistence and diagram; runtime/hardware/source editing unproven" if feature["engine"] == "MONITOR" else "typed device drafts, graphical XKB/curve forms, static Lua import/export and schema-pinned persistence; hardware/source/runtime conformance unproven" if feature["engine"] == "DEVICE" else "typed binding/submap drafts, dispatcher variants, focused recording, static projection and schema-pinned storage; selector/workflow/source/runtime conformance unproven" if feature["engine"] == "BIND" else "typed workspace drafts, static filter builders, source merge findings and schema-pinned persistence; complete selector/layout/workflow/source/runtime conformance unproven" if feature["engine"] == "WORKSPACE" else "typed window/layer rule declarations, consumed predicates, isolated RE2 samples, geometry inputs, group-aware export and schema-pinned storage; complete semantic builders/source/runtime conformance unproven" if feature["engine"] == "RULE" else "typed animation node/curve drafts, source-derived inheritance/timing, graphical controls, ordered static projection and schema-pinned storage; full source/style/runtime/preview conformance unproven" if feature["engine"] == "ANIMATION" else "typed gesture drafts, ordered registration/unset identity, static phase action blocks, synthetic callback payloads and schema-pinned storage; full workflow/source/input/runtime/preview conformance unproven" if feature["engine"] == "GESTURE" else "no dedicated feature implementation yet",
            "activation_certified": False, "preview_certified": False,
        })
    sections = numbered_sections(BLUEPRINT)
    orders = table_rows(BLUEPRINT, r"E\d\d")
    stages = table_rows(PLAN, r"[1-9]\. .+")
    invariants = model_invariants()
    if (len(sections), len(orders), len(security), len(stages), len(invariants)) != (24, 12, 24, 9, 10):
        raise ValueError("Audit structure changed: reconcile the completeness register explicitly")
    candidates = [{"id": f"documentation-candidate-{index:04}", "source": row,
        "classification": "pending", "complete": False}
        for index, row in enumerate(contracts["documentation_candidates"], 1)]
    foundations = [
        {"area": "source dependency capture and ordered evaluation", "code": ["crates/editor-core/src/module_path.rs", "crates/editor-core/src/module_cache.rs", "crates/editor-core/src/lua/dependencies.rs", "crates/editor-core/src/lua/replay.rs", "crates/editor-core/src/lua/replay/eval.rs", "crates/editor-core/native/lua_syntax.cpp", "crates/editor-core/src/source_scope.rs", "crates/editor-core/src/source_scope/probe.rs", "data/lua-profiles.json", "crates/editor-app/src/capture_worker.rs", "crates/editor-app/src/directory_worker.rs", "crates/editor-app/src/fd_channel.rs", "crates/editor-app/src/source_modules.rs", "crates/editor-app/src/source_access.rs", "crates/editor-app/src/source_graph.rs", "crates/editor-app/src/source_analysis.rs", "crates/editor-app/src/source_context.rs"],
         "test_references": ["scripts/source_graph_probe.py", "scripts/module_load_probe.py", "scripts/ordered_module_probe.py", "scripts/lua_profile_probe.py", "scripts/runtime_path_probe.py", "actual_graph_capture_feeds_ordered_cache_and_deferred_file_loading_without_executing_lua", "actual_graph_records_raw_status_then_captures_the_distinct_normalized_load_target", "actual_graph_keeps_hardlinked_module_load_views_and_cache_keys_distinct", "unknown_inherited_path_provenance_survives_aliases_and_literal_replacement", "actual_metadata_helper_separates_absence_from_escape_denial_and_cycle", "changed_link_or_scope_invalidates_a_pending_negative_observation", "ordered_reference_observations", "loader_edges_and_api_calls_share_actual_order_and_document_provenance", "ordered_modules_and_compilation_run_inside_the_restricted_worker", "actual_policy_allows_selected_read_and_denies_other_read_write_network_and_process_creation", "actual_capability_transport_retains_approved_folder_and_refuses_external_targets", "contained_relative_absolute_and_alias_paths_preserve_bytes_and_link_evidence", "unavailable_containment_or_landlock_refuses_folder_capture_without_fallback", "malformed_marker_multiple_and_truncated_descriptors_are_refused", "encoded_retention_budget_refuses_atomically_and_preview_bounds_unicode_and_controls", "context_preserves_unknown_empty_and_byte_paths_and_builder_preserves_templates", "test_review"],
         "limits": "source-pinned plans/cache transitions, syntactic dependency sites, isolated selected-file and held-folder capture, typed metadata-only point observations separating absence from blocking uncertainty, bounded temporary GUI snapshots/analysis and cleanup, captured-resolution coordination with per-edge sequential evidence and separate load views, focused graphical files/calls inspection, supplied-context folder/template editor without added read authority, explicit compiler/library profiles with four-release reference checks, ordered immutable-byte evaluation under supplied/captured resolution contexts with explicit unknowns and compilation-only validation; no editable graph import, complete language/native/API codecs, ownership/alias/mount/concurrency certification, wildcard enumeration, graph save/reopen/history/reconciliation, source insertion/deletion, effective discovery, isolated preview, apply/recovery or resource/release acceptance"},
        {"area": "gesture draft editor", "code": ["crates/editor-core/src/gesture.rs", "crates/editor-app/src/gestures.rs", "data/gesture-semantics.json", "data/gesture-source-fixtures.json"],
         "test_references": ["source_registration_and_callback_payload_observations_match_the_models", "callbacks_preserve_phase_presence_parameters_preparation_and_legacy_shape", "test_editor", "gestures_survive_application_reopen_with_edits_and_reject_schema_and_shape_tampering", "scripts/gesture_source_probe.py"],
         "limits": "nine declaration fields, three phase blocks and five-part unset identity, ordered registration findings, static dispatcher workflows, synthetic callback data, private persistence and undo; no complete event-dependent workflows, source editing, effective discovery, input routing/inhibition/renderer/timeout conformance, isolated preview, guarded apply/recovery or resource/release certification"},
        {"area": "animation draft editor", "code": ["crates/editor-core/src/animation.rs", "crates/editor-app/src/animations.rs", "data/animation-semantics.json", "data/animation-source-fixtures.json"],
         "test_references": ["every_source_node_round_trips_and_whole_node_overrides_have_required_enabled", "static_models_match_independent_source_binding_tree_observations", "test_editor", "animation_declarations_survive_restricted_application_reopen_and_refuse_schema_tampering", "scripts/animation_source_probe.py"],
         "limits": "34 public nodes and one internal node per snapshot, ordered node/Bézier/spring drafts, inherited whole-node resolution, duration explanation, style presets, curve diagrams/dragging, private persistence and undo; no full numeric/style/source/runtime conformance, compositor preview, live apply/recovery, accessibility or resource/release certificates"},
        {"area": "window/layer rule draft editor", "code": ["crates/editor-core/src/rule.rs", "crates/editor-core/src/regex.rs", "crates/editor-core/native/regex.cpp", "crates/editor-app/src/rules.rs", "crates/editor-app/src/rule_forms.rs", "crates/editor-core/src/rule_codec.rs", "data/rule-codec-provenance.json", "data/rule-codec-fixtures.json", "data/window-effects.json", "data/layer-effects.json", "data/rule-semantics.json"],
         "test_references": ["all_audited_effects_and_predicates_round_trip_without_executing_expressions", "named_calls_reset_enabled_replace_supplied_matches_append_groups_and_keep_registration_order", "test_editor", "actual_restricted_worker_projects_inert_rule_groups_and_preserves_copy", "rule_declarations_survive_application_reopen_without_accessing_original_and_refuse_schema_tampering", "scripts/rule_source_probe.py", "typed_codecs_agree_with_pinned_primitive_parser_observations", "test_forms", "scripts/rule_codec_source_probe.py"],
         "limits": "57/60 window-effect controls, ten layer effects, consumed conditions, isolated exact-text/RE2 samples, geometry inputs, group boundaries, explicit named ordering, private persistence/undo and specialized opacity/fullscreen/group/event/mode/border controls; no complete geometry/style/selector or exotic/escaped literal domains, named-object lifecycle, effective rule discovery, source editing, compositor conformance, preview/apply/recovery or release certification"},
        {"area": "workspace draft editor", "code": ["crates/editor-core/src/workspace_rule.rs", "crates/editor-core/src/workspace_selector.rs", "crates/editor-app/src/workspaces.rs", "crates/editor-app/src/selector_forms.rs", "data/workspaces.json", "data/workspace-semantics.json"],
         "test_references": ["every_audited_workspace_field_round_trips_as_an_explicit_override", "repeated_selectors_follow_merge_left_and_disabled_declarations_do_not_disable_prior_rule", "test_editor", "actual_restricted_worker_projects_workspace_rules_without_running_empty_hooks", "workspace_declarations_survive_application_reopen_and_schema_tampering_is_refused", "scripts/workspace_source_probe.py"],
         "limits": "15 field controls, addresses/static filter clauses, enabled/unset, layout choices and retained options, source consolidation findings, private persistence and undo; no complete selector/layout/workflow codecs, source editing, live creation/matching, apply/recovery or preview certification"},
        {"area": "binding/submap draft editor", "code": ["crates/editor-core/src/action.rs", "crates/editor-core/src/binding.rs", "crates/editor-app/src/bindings.rs", "crates/editor-app/src/bind_forms.rs", "data/actions.json", "data/bindings.json"],
         "test_references": ["every_constructor_variant_exports_and_imports_its_typed_arguments", "test_editor", "binding_actions_remain_inert_in_actual_restricted_worker_and_preserve_timing", "application_reopens_pinned_binding_graph_and_rejects_tampering", "duplicate_submap_rebases_subtree_transitions_but_keeps_external_targets"],
         "limits": "51/52 public dispatcher constructor forms, chords/flags, focused recording, literal submap rename/duplication and escape drafts, static findings, private persistence and inverse undo; no complete selector/workflow builders, effective conflicts/device intersections, source editing, live delivery, apply/recovery or compositor preview certification"},
        {"area": "device draft editor", "code": ["crates/editor-core/src/device.rs", "crates/editor-core/src/xkb.rs", "crates/editor-app/src/devices.rs", "crates/editor-app/src/device_forms.rs", "data/devices.json"],
         "test_references": ["every_device_field_round_trips_the_pinned_static_lua_codec", "test_editor", "actual_worker_projects_devices_and_rejects_external_keyboard_entities", "device_drafts_survive_application_reopen_and_schema_drift_is_rejected"],
         "limits": "43 fields and name selector, author category hints, linked XKB choices from installed metadata, numeric acceleration/scroll builders, ordered drafts and private persistence; no hardware capability discovery, effective global/default merge, tag-to-bind analysis, keyboard compilation, source editing, live input recovery or activation/preview certificates"},
        {"area": "monitor draft editor", "code": ["crates/editor-core/src/monitor.rs", "crates/editor-app/src/monitors.rs", "data/monitors.json"],
         "test_references": ["every_audited_monitor_field_has_a_typed_export_and_static_import", "test_editor", "monitor_drafts_survive_actual_application_reopen_without_replacing_them_with_source", "actual_restricted_worker_projects_monitors_and_keeps_unknown_calls_inert"],
         "limits": "22 versioned field controls and output selector, ordered drafts, explicit logical geometry, static Lua import/export and private saved snapshots; no hardware detection, effective rule resolution, source-span mutation/reconciliation, ICC decoding, monitor recovery, live apply or compositor preview certification"},
        {"area": "scalar draft forms", "code": ["crates/editor-app/src/controls.rs", "crates/editor-core/src/lib.rs"],
         "test_references": ["tests::native_forms_and_draft_workflow", "lua::tests::exported_drafts_round_trip_all_types"],
         "limits": "initial values for 755 forms; not full per-field conformance, runtime activation or accessibility certification"},
        {"area": "static Lua and parser isolation", "code": ["crates/editor-core/src/lua.rs", "crates/editor-app/src/worker.rs"],
         "test_references": ["filesystem_and_network_are_denied", "opaque_commands_cannot_create_files"],
         "limits": "restricted source projection and existing literals; no include/scope completeness or preview isolation proof"},
        {"area": "private source-backed draft snapshots", "code": ["crates/editor-core/src/source.rs", "crates/editor-core/src/source_path.rs", "crates/editor-core/src/workspace.rs", "crates/editor-core/src/reconcile.rs", "crates/editor-app/src/workflow.rs", "crates/editor-app/src/reconciliation.rs", "crates/editor-app/src/history.rs"],
         "test_references": ["application_reparses_snapshot_and_preserves_edits_without_opening_recorded_paths", "application_rejects_bad_hash_forged_reports_and_syntax_in_saved_source", "test_source_persistence", "relative_absolute_and_parent_links_preserve_targets_and_chain", "identical_bytes_through_a_replaced_link_change_recorded_identity", "conflicting_edit_and_delete_require_explicit_choices", "application_compares_changed_and_opaque_sources_with_restricted_parser_and_no_writes", "test_review"],
         "limits": "single Lua root, symlink-chain evidence and reviewed static revision reconciliation; no generated ownership/include graph, multi-file merge, persistent history/checkpoint journal or watched-path certification"},
        {"area": "read-only selected-session inspection", "code": ["crates/editor-discovery/src/lib.rs"],
         "test_references": ["selected_session_only_uses_fixed_read_queries", "rejects_path_injection_symlinks_and_replaced_session"],
         "limits": "version/descriptions fixtures; no actual host compositor certificate, other inspectors or event subscription"},
    ]
    for foundation in foundations:
        if not all((ROOT / code).is_file() for code in foundation["code"]):
            raise ValueError(f"Missing foundation code: {foundation['area']}")
    return {
        "format": "hyprland-editor-completion-register-1", "full_goal_complete": False,
        "meaning": "Obligation tracking, not a test result or release certificate. All full contracts remain unproven; foundation code/test references identify only narrower work.",
        "audit_pins": {"baseline": contracts["baseline_commit"], "audited_main": contracts["current_commit"]},
        "inputs": {str(path.relative_to(ROOT)): digest(path.read_bytes()) for path in [CONTRACTS, BLUEPRINT, PLAN]},
        "foundations": foundations, "blueprint_sections": sections, "engineering_orders": orders,
        "security_gates": security, "implementation_stages": stages, "model_invariants": invariants,
        "feature_contracts": obligations, "documentation_candidates": candidates,
        "remaining_verification": [
            "Every per-version field, variant, alias, parser, observation, activation and preview contract",
            "All threat-table fixtures, grammar/event fuzzing, failure/race/crash injection and helper/lab teardown",
            "Exact upstream parser and behavior suites for both pinned packs and supported distro/build profiles",
            "Independent hardware results for monitor/input recovery, hotplug, DPI, transformations, VRR/HDR and GPUs",
            "Accessibility, localization, source ownership, corrupted-state recovery, packaging, dependencies and signed releases",
            "Complete classification of documentation and adopted ecosystem adapter inventories; upstream registration diffs",
            "Repeatable memory/CPU/long-session measurements rather than a single idle startup sample",
        ],
    }


def markdown(report):
    counts = Counter(row["engine"] for row in report["feature_contracts"])
    text = """# Full-project completion register

The goal remains the complete editor described in the audit and blueprint. This register preserves the full scope, including all 1,453 feature contracts, 756 documentation candidates, 24 blueprint sections, 12 engineering work orders, 24 mandatory security gates, nine implementation stages and ten shared model invariants.

**The project is not complete.** None of the full feature contracts has a release-wide certificate. The working foundations are narrower than those contracts. Code/test references identify evidence to inspect; a reference does not establish that a test ran, that it covers a whole requirement or that a physical behavior was certified.

The [machine-readable register](completion-register.json) retains every contract ID, version membership, required editor/codec and named test gate. It also retains the source and exact text hashes for blueprint section scopes. Pending documentation candidates are not silently counted as implemented settings. The existing [implementation status](implementation-status.md) supplies the next work sequence.

| Engine | Full contracts | Full acceptance |
| --- | ---: | --- |
"""
    text += "".join(f"| {engine} | {count} | Not proven |\n" for engine, count in sorted(counts.items()))
    text += "\n## Current foundations and limits\n\n"
    for foundation in report["foundations"]:
        text += f"- **{foundation['area']}**: {foundation['limits']}.\n"
    text += """
## Maintaining the register

Run `python3 scripts/implementation_report.py` after changing the audit manifest or blueprint. CI runs `--check` to reject drift. This check only proves register consistency with the pinned audit; it does not certify upstream completeness, feature behavior or release safety. Future certification records must bind an exact source/schema/binary/profile identity to requirement-specific execution results before a full contract advances beyond Specified.

The source persistence milestone does not close E03/E05: lossless include/symlink graphs, full multi-file changed-revision conflict handling, actual watched-path isolation, source insertion/deletion and recovery still need implementation and verification. Green ordinary tests cannot substitute for those gates.
"""
    return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    report = build()
    outputs = {
        ROOT / "docs/completion-register.json": json.dumps(report, indent=2, ensure_ascii=False) + "\n",
        ROOT / "docs/completion-register.md": markdown(report),
    }
    for path, text in outputs.items():
        if args.check:
            if not path.exists() or path.read_text() != text:
                raise SystemExit(f"Completion register is stale: {path.relative_to(ROOT)}")
        else:
            path.write_text(text)
    print(f"Tracked {len(report['feature_contracts'])} full contracts; complete-project acceptance remains unproven")


if __name__ == "__main__":
    main()
