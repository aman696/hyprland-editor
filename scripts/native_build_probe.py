#!/usr/bin/env python3
"""Check exact audited source build requests, independently of model assumptions.

This checks source metadata, not an installed compositor's compiler, dependencies
or compatibility. Source bodies stay in memory and no repository is retained.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/native-build-profiles.json'
# Reviewed against GitHub's immutable commit/blob API and raw source.
PINS = {
    'baseline': ('efb50993780079460b0cbed1363e2166a2de1d9f', {
        'CMakeLists.txt': ('cf7f53aa03e1de2e3ef5e4a475249c1895214bb2', '97b8028aa342cf6ee6f04b5db2e67e6e4fe6adf5d3fd67bad18005193db36516'),
        'VERSION': ('4425c37f1ff277ab4ae329affbfe3cc72ac9d180', '83983962d9161a0ec80cc585a3d3a1deb1f7e6c5ed8a5ba806f7dc338ebf790f')}),
    'current': ('6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7', {
        'CMakeLists.txt': ('4318f1a31dbdc95238f061f1bafc9af867825d1c', '92bd722255442d84c52781e696eff1a75b65652bcb4c19fd7502c7c9378cbf16'),
        'VERSION': ('2f4c74eb2ddac0be8e75784c1ee8b00e3a83f8f2', '3fea81d177087f5d3380893d95b86573a803b34ed45419ec381bdd776f526cee')}),
}


def fetch(version, commit, path, expected):
    url = f'https://raw.githubusercontent.com/hyprwm/Hyprland/{commit}/{path}'
    with urllib.request.urlopen(url, timeout=30) as response:
        data = response.read(256 * 1024 + 1)
    if len(data) > 256 * 1024:
        raise RuntimeError('Source build metadata exceeds its bound')
    blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    sha = hashlib.sha256(data).hexdigest()
    if (blob, sha) != expected:
        raise RuntimeError(f'Unreviewed source build metadata: {version}/{path}')
    return version, path, data.decode(), {'url': url, 'git_blob': blob, 'sha256': sha}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    contracts = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    api = next(f for f in contracts['features'] if f['id'] == 'api:hl.workspace_rule')
    for version, (commit, _) in PINS.items():
        if f'/blob/{commit}/' not in api['handler_versions'][version]['source']:
            raise RuntimeError('Native build metadata and API snapshot differ')
    tasks = [(v, c, p, pin) for v, (c, paths) in PINS.items() for p, pin in paths.items()]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        rows = list(pool.map(lambda x: fetch(*x), tasks))
    snapshots = []
    for version, (commit, _) in PINS.items():
        files = {p: (text, evidence) for v, p, text, evidence in rows if v == version}
        cmake, source = files['CMakeLists.txt']
        cpp = re.findall(r'^set\(CMAKE_CXX_STANDARD\s+(\d+)\)', cmake, re.M)
        required = re.findall(r'^set\((CXX_STANDARD_REQUIRED)\s+(ON)\)', cmake, re.M)
        lua = re.findall(r'^pkg_search_module\(LUA REQUIRED IMPORTED_TARGET GLOBAL ([^\n)]+)\)', cmake, re.M)
        if len(cpp) != 1 or len(required) != 1 or len(lua) != 1:
            raise RuntimeError('Native build declaration needs review')
        snapshots.append({'version': version, 'commit': commit,
                          'source_version': files['VERSION'][0].strip(),
                          'version_source': files['VERSION'][1], 'build_source': source,
                          'requested_cpp_standard': int(cpp[0]),
                          'required_flag_assignment': {'variable': required[0][0], 'value': required[0][1]},
                          'lua_pkgconfig_candidates': lua[0].split(),
                          'lua_selection': 'first_successful_alternative_not_combined_version_range',
                          'selected_compiler_observed': False, 'selected_runtime_library_observed': False})
    content = {'format': 'hye-native-build-profiles-1',
               'scope': 'Exact source CMake/version declarations; actual resolved compiler, standard-library behavior, linked Lua and selected compositor remain unobserved. pkg_search_module candidates are alternatives, not a combined Lua version range. CXX_STANDARD_REQUIRED is recorded as the source assignment, not a guarantee of effective target-property enforcement.',
               'snapshots': snapshots}
    rendered = json.dumps(content, indent=2) + '\n'
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != rendered:
            raise SystemExit('Audited native build profile drift')
    else:
        OUTPUT.write_text(rendered)
    print('Verified source C++26 request and ordered Lua search alternatives; no strict Lua version range or selected runtime is inferred')


if __name__ == '__main__':
    main()
