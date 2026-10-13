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
                if feature["kind"] == "scalar" else "no dedicated feature implementation yet",
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
        {"area": "scalar draft forms", "code": ["crates/editor-app/src/controls.rs", "crates/editor-core/src/lib.rs"],
         "test_references": ["tests::native_forms_and_draft_workflow", "lua::tests::exported_drafts_round_trip_all_types"],
         "limits": "initial values for 755 forms; not full per-field conformance, runtime activation or accessibility certification"},
        {"area": "static Lua and parser isolation", "code": ["crates/editor-core/src/lua.rs", "crates/editor-app/src/worker.rs"],
         "test_references": ["filesystem_and_network_are_denied", "opaque_commands_cannot_create_files"],
         "limits": "restricted source projection and existing literals; no include/scope completeness or preview isolation proof"},
        {"area": "private source-backed draft snapshots", "code": ["crates/editor-core/src/source.rs", "crates/editor-core/src/source_path.rs", "crates/editor-core/src/workspace.rs", "crates/editor-app/src/workflow.rs"],
         "test_references": ["application_reparses_snapshot_and_preserves_edits_without_opening_recorded_paths", "application_rejects_bad_hash_forged_reports_and_syntax_in_saved_source", "test_source_persistence", "relative_absolute_and_parent_links_preserve_targets_and_chain", "identical_bytes_through_a_replaced_link_change_recorded_identity"],
         "limits": "single Lua root and explicit symlink-chain evidence in GUI; no generated ownership or include graph, changed-revision merge, checkpoint journal or watched-path certification"},
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

The source persistence milestone does not close E03/E05: lossless include/symlink graphs, changed-revision conflict handling, actual watched-path isolation, source insertion/deletion and recovery still need implementation and verification. Green ordinary tests cannot substitute for those gates.
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
