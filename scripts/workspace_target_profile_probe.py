#!/usr/bin/env python3
"""Build hash-pinned target Lua releases privately and check workspace profiles.

Sources/libraries/pkg-config entries/fixtures stay in one temporary scope. Only
trusted fixture Lua executes. No system packages or configuration are installed.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
# Official https://www.lua.org/ftp/ SHA-256 entries, reviewed 24 October 2026.
RELEASES = {
    '5.5.0': '57ccc32bbbd005cab75bcc52444052535af691789dba2b9016d5c50640d68b3d',
    '5.5.1': '1c4b4068d67061f2a2231ad2b5422e77acea1487ea9890f6320af614f4373dce',
}


def unpack(data, root, release):
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        if len(members) > 512 or sum(m.size for m in members) > 8 * 1024 * 1024:
            raise RuntimeError('Lua source extraction exceeds its bounds')
        for member in members:
            path = PurePosixPath(member.name)
            if path.is_absolute() or '..' in path.parts or not path.parts or path.parts[0] != 'lua-' + release or not (member.isfile() or member.isdir()) or member.size > 2 * 1024 * 1024:
                raise RuntimeError('Unexpected Lua archive entry')
        archive.extractall(root, members=members, filter='data')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', choices=sorted(RELEASES), action='append')
    parser.add_argument('--observation', type=Path)
    parser.add_argument('--compiler', default='c++')
    parser.add_argument('--cpp26-standard', choices=['c++26', 'c++2c'], default='c++2c')
    args = parser.parse_args()
    cflags = subprocess.check_output(['pkg-config', '--cflags', 'lua5.4'], text=True).strip()
    libs = subprocess.check_output(['pkg-config', '--libs', 'lua5.4'], text=True).strip()
    legacy = subprocess.check_output(['pkg-config', '--modversion', 'lua5.4'], text=True).strip()
    observations = []
    for release in args.release or sorted(RELEASES):
        with tempfile.TemporaryDirectory(prefix='hye-workspace-target-') as directory:
            root = Path(directory)
            url = f'https://www.lua.org/ftp/lua-{release}.tar.gz'
            with urllib.request.urlopen(url, timeout=30) as response:
                data = response.read(1024 * 1024 + 1)
            if len(data) > 1024 * 1024 or hashlib.sha256(data).hexdigest() != RELEASES[release]:
                raise RuntimeError('Official target Lua archive identity differs')
            unpack(data, root, release)
            source = root / ('lua-' + release) / 'src'
            subprocess.run(['make', '-C', str(source), '-j2', 'liblua.a', 'MYCFLAGS=-fPIC -DLUA_USE_LINUX'], check=True, stdout=subprocess.DEVNULL, timeout=90)
            library = hashlib.sha256((source / 'liblua.a').read_bytes()).hexdigest()
            (root / 'lua5.5.pc').write_text(f'Name: Lua\nDescription: private hash-pinned workspace reference\nVersion: {release}\nCflags: -I{source}\nLibs: {source}/liblua.a -lm -ldl\n')
            (root / 'lua5.4.pc').write_text(f'Name: Lua\nDescription: existing legacy reference\nVersion: {legacy}\nCflags: {cflags}\nLibs: {libs}\n')
            observed = root / 'observation.json'
            subprocess.run([sys.executable, '-B', str(ROOT / 'scripts/workspace_runtime_probe.py'), '--check', '--compiler', args.compiler, '--cpp26-standard', args.cpp26_standard, '--observation', str(observed)], env=dict(os.environ, PKG_CONFIG_LIBDIR=str(root), PKG_CONFIG_PATH=''), check=True, cwd=ROOT, timeout=300)
            value = json.loads(observed.read_text())
            for profile in value['profiles']:
                if profile['lua_family'] == '5.5':
                    major, minor, patch = map(int, release.split('.'))
                    if profile['header_release'] != major * 10000 + minor * 100 + patch:
                        raise RuntimeError('Target reference did not use the requested private release')
            observations.append({'release': release, 'archive_url': url, 'archive_sha256': RELEASES[release], 'library_sha256': library, 'matrix': value})
            print(f'Lua {release}: private source build and workspace profile matrix passed; temporary source/builds removed on scope exit', flush=True)
    content = {'format': 'hye-workspace-target-release-observation-1', 'profiles': observations, 'selected_compositor_observed': False}
    if args.observation:
        args.observation.write_text(json.dumps(content, indent=2) + '\n')
    print(f'Verified {len(observations)} private target releases; sources/builds removed; application worker compatibility remains unfinished')


if __name__ == '__main__':
    main()
