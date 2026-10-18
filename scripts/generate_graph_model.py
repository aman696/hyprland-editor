#!/usr/bin/env python3
"""Pin saved graph reconstruction to its bundled model inputs without embedding Rust sources."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "data/graph-model.json"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = list((ROOT / "crates/editor-core/src").rglob("*.rs"))
    paths += [ROOT / path for path in [
        "Cargo.toml", "Cargo.lock", "crates/editor-core/Cargo.toml", "crates/editor-app/Cargo.toml", "crates/editor-core/build.rs", "crates/editor-core/native/lua_syntax.cpp",
        "crates/editor-app/src/source_graph.rs", "crates/editor-app/src/worker.rs",
        "crates/editor-app/src/process_policy.rs", "data/source-graph-semantics.json",
        "data/lua-profiles.json", "data/scalar-semantics.json", "data/scalars.json",
        "crates/editor-core/native/complex_text.cpp", "data/complex-scalar-sources.json", "data/complex-scalar-fixtures.json",
    ]]
    value = {"format": "hye-graph-model-source-1", "inputs": {
        str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(paths)
    }}
    text = json.dumps(value, indent=2) + "\n"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != text:
            raise SystemExit("Saved graph model pin is stale: run scripts/generate_graph_model.py")
    else:
        OUTPUT.write_text(text)
    print(f"Pinned {len(paths)} source/model inputs for saved graph reconstruction")


if __name__ == "__main__":
    main()
