#!/usr/bin/env python3
"""Compare trusted source/parser references under an explicit Lua 5.5 library.

The caller supplies a private pkg-config scope. No user configuration executes.
Matching the finite reference corpus does not enable the application evaluator.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import workspace_runtime_probe as runtime

ROOT = Path(__file__).resolve().parents[1]
JOBS = [
    ('ordered_module_probe.py', 'ordered-module-fixtures.json'),
    ('runtime_path_probe.py', 'runtime-path-fixtures.json'),
    ('scalar_value_probe.py', 'scalar-value-fixtures.json'),
    ('complex_scalar_probe.py', 'complex-scalar-fixtures.json'),
    ('lua55_language_probe.py', 'lua55-language-fixtures.json'),
]


def corpus():
    return {name: hashlib.sha256((ROOT / 'data' / name).read_bytes()).hexdigest() for _, name in JOBS}


def collect(job, args, standard, root):
    tool, name = job
    output = root / name
    result = subprocess.run([sys.executable, '-B', str(ROOT / 'scripts' / tool), '--lua-package', 'lua5.5', '--compiler', args.compiler, '--cpp-standard', standard, '--output', str(output)], cwd=ROOT, capture_output=True, text=True, timeout=150, env=dict(os.environ, LC_ALL='C'))
    if result.returncode:
        raise RuntimeError(f'{tool} refused: {result.stdout} {result.stderr}')
    actual = json.loads(output.read_text())
    expected = json.loads((ROOT / 'data' / name).read_text())
    if 'meaning' in expected:
        expected['meaning'] = expected['meaning'].replace('Lua 5.4', 'Lua 5.5')
    if actual != expected:
        # Do not accept an automatic override/delta just because generation ran.
        raise RuntimeError(f'Lua 5.5 differs from the retained source reference: {name}')
    if 'cases' in actual:
        cases = len(actual['cases'])
    else:
        cases = len(actual['search']) + len(actual['normalization'])
    return name, {'cases': cases, 'observations_sha256': hashlib.sha256(output.read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', choices=['5.5.0', '5.5.1'], required=True)
    parser.add_argument('--compiler', default='c++')
    parser.add_argument('--cpp26-standard', choices=['c++26', 'c++2c'], default='c++2c')
    parser.add_argument('--observation', type=Path)
    args = parser.parse_args()
    package_release = subprocess.check_output(['pkg-config', '--modversion', 'lua5.5'], text=True).strip()
    if package_release != args.release:
        raise RuntimeError('Private source reference package release differs from request')
    tool_names = [tool for tool, _ in JOBS] + ['native_fixture_context.py', 'lua55_source_probe.py', 'workspace_runtime_probe.py', 'module_load_probe.py', 'source_graph_probe.py', 'scalar_registration_probe.py']
    tools = {name: hashlib.sha256((ROOT / 'scripts' / name).read_bytes()).hexdigest() for name in tool_names}
    references = corpus()
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.5'], text=True))
    profiles = []
    with tempfile.TemporaryDirectory(prefix='hye-lua55-source-') as directory:
        root = Path(directory)
        for label, standard in [(23, 'c++23'), (26, args.cpp26_standard)]:
            folder = root / str(label)
            folder.mkdir()
            profiler = runtime.compile_fixture(folder, 'profile', runtime.PROFILE, flags, standard, args.compiler)
            profile = json.loads(subprocess.check_output([str(profiler)], text=True, timeout=4, env={'LC_ALL': 'C'}))
            major, minor, patch = map(int, args.release.split('.'))
            if profile['header_release'] != major * 10000 + minor * 100 + patch or profile['runtime_abi'] != 505 or f'Lua {args.release} ' not in bytes.fromhex(profile['identity_hex']).decode():
                raise RuntimeError('Source reference header/runtime identity differs')
            if (label == 26 and profile['cplusplus'] <= 202302) or (label == 23 and not 202100 <= profile['cplusplus'] <= 202302):
                raise RuntimeError('Source reference language mode differs')
            with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
                rows = dict(pool.map(lambda job: collect(job, args, standard, folder), JOBS))
            profile.update(requested_cpp_standard=label, references=rows, profiler_sha256=hashlib.sha256(profiler.read_bytes()).hexdigest())
            profiles.append(profile)
            print(f'Lua {args.release}/C++{label}: {sum(r["cases"] for r in rows.values())} source/parser/path reference observations match; application integration remains unverified', flush=True)
    if references != corpus() or tools != {name: hashlib.sha256((ROOT / 'scripts' / name).read_bytes()).hexdigest() for name in tool_names}:
        raise RuntimeError('Source reference inputs changed during observation')
    value = {'format': 'hye-lua55-source-observation-1', 'release': args.release, 'profiles': profiles, 'reference_inputs': references, 'tools': tools, 'application_worker_verified': False, 'complete_language_verified': False, 'selected_compositor_observed': False}
    if args.observation:
        args.observation.write_text(json.dumps(value, indent=2) + '\n')
    print('Source reference comparisons passed; temporary outputs/builds removed')


if __name__ == '__main__':
    main()
