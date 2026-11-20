#!/usr/bin/env python3
"""Exercise source-declared CMake search order using private synthetic PC files.

No libraries, toolchains, compositor, installed packages or host configuration
are selected or changed. This checks search behavior only.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CASES = [
    ('legacy_generic_fallback', {'lua': '5.4.9'}, ('lua', '5.4.9')),
    ('preferred_generic', {'lua': '5.5.1'}, ('lua', '5.5.1')),
    ('later_generic_has_no_combined_upper_bound', {'lua': '5.6.0'}, ('lua', '5.6.0')),
    ('named_alias_has_no_version_constraint', {'lua55': '5.4.9', 'lua': '5.5.1'}, ('lua55', '5.4.9')),
    ('named_order', {'lua55': '5.5.0', 'lua5.5': '5.5.1'}, ('lua55', '5.5.0')),
    ('required_absence', {}, None),
]


def main():
    data = json.loads((ROOT / 'data/native-build-profiles.json').read_text())
    count = 0
    for row in data['snapshots']:
        candidates = row['lua_pkgconfig_candidates']
        if candidates != ['lua55', 'lua5.5', 'lua-55', 'lua-5.5', 'lua>=5.5', 'lua<5.6']:
            raise RuntimeError('Unreviewed Lua search alternatives')
        for name, packages, expected in CASES:
            with tempfile.TemporaryDirectory(prefix='hye-native-selection-') as directory:
                root = Path(directory)
                pc = root / 'pc'
                pc.mkdir()
                for package, version in packages.items():
                    (pc / (package + '.pc')).write_text(f'Name: {package}\nDescription: private search fixture\nVersion: {version}\nLibs:\nCflags:\n')
                (root / 'CMakeLists.txt').write_text('cmake_minimum_required(VERSION 3.19)\nproject(SearchFixture LANGUAGES NONE)\nset(PKG_CONFIG_USE_CMAKE_PREFIX_PATH FALSE)\nfind_package(PkgConfig REQUIRED)\npkg_search_module(LUA REQUIRED IMPORTED_TARGET GLOBAL ' + ' '.join(candidates) + ')\nfile(WRITE "${CMAKE_BINARY_DIR}/selected.txt" "${LUA_MODULE_NAME}|${LUA_VERSION}")\n')
                result = subprocess.run(['cmake', '-S', str(root), '-B', str(root / 'build')], env=dict(os.environ, PKG_CONFIG_LIBDIR=str(pc), PKG_CONFIG_PATH=''), capture_output=True, text=True, timeout=15)
                selected = root / 'build/selected.txt'
                if expected is None:
                    if result.returncode == 0 or (selected.exists() and selected.read_text() != '|'):
                        raise RuntimeError(f'Required missing package unexpectedly selected: {name}: {result.stdout} {result.stderr}; value={selected.read_text() if selected.exists() else None}')
                    if 'None of the required' not in result.stderr:
                        raise RuntimeError(f'Unexpected CMake refusal: {name}: {result.stderr}')
                elif result.returncode or not selected.exists() or tuple(selected.read_text().split('|')) != expected:
                    raise RuntimeError(f'Unexpected CMake selection: {name}: {result.stdout} {result.stderr}')
                count += 1
    print(f'Verified {count} private CMake selections/refusals; alternatives do not establish a strict Lua range or selected compositor ABI; temporary files removed')


if __name__ == '__main__':
    main()
