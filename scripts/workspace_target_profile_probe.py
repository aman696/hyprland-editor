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
import re
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


def check_static_parent(root, release, compiler, installed=False):
    host = subprocess.check_output(['rustc', '-vV'], text=True).split('host: ', 1)[1].splitlines()[0]
    if host not in ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu']:
        raise RuntimeError('Private static compiler comparison requires an explicit native GNU profile')
    target = root / 'static-target'
    env = dict(os.environ, PKG_CONFIG_LIBDIR=str(root), PKG_CONFIG_PATH='', CARGO_TARGET_DIR=str(target), HYE_LUA55_TEST_RELEASE=release, CXX=compiler, RUSTFLAGS=f'-C target-feature=+crt-static -C linker={compiler}')
    env.pop('CARGO_ENCODED_RUSTFLAGS', None)
    subprocess.run(['cargo', 'test', '--locked', '--release', '--target', host, '-p', 'hyprland-editor-lua55', '--features', 'native55', '--', '--test-threads=2'], cwd=ROOT, env=env, check=True, timeout=240)
    image = target / host / 'release/hyprland-editor-lua55'
    headers = subprocess.check_output(['readelf', '-l', '-d', str(image)], text=True, timeout=10)
    if 'INTERP' in headers or '(NEEDED)' in headers:
        raise RuntimeError('Static compiler image still depends on a host interpreter/library')
    image_sha = hashlib.sha256(image.read_bytes()).hexdigest()
    parent_env = dict(os.environ, CARGO_TARGET_DIR=str(root / 'parent-target'), HYE_COMPILER_CLIENT_IMAGE=str(image), HYE_COMPILER_CLIENT_SHA256=image_sha, HYE_COMPILER_CLIENT_RELEASE=release)
    built = subprocess.check_output(['cargo', 'test', '--locked', '-p', 'hyprland-editor', '--test', 'compiler_client', '--no-run', '--message-format=json'], cwd=ROOT, env=parent_env, text=True, timeout=240)
    artifacts = [json.loads(line) for line in built.splitlines() if line.strip()]
    binaries = {v['executable'] for v in artifacts if v.get('reason') == 'compiler-artifact' and v.get('target', {}).get('name') == 'compiler_client' and v.get('executable')}
    if len(binaries) != 1:
        raise RuntimeError('Private compiler parent test image is ambiguous')
    parent = Path(binaries.pop())
    result = subprocess.run([str(parent), '--ignored', '--exact', 'actual_static_worker_parent_preserves_identity_and_never_executes_source', '--test-threads=1'], cwd=ROOT, env={k:parent_env[k] for k in ['HYE_COMPILER_CLIENT_IMAGE','HYE_COMPILER_CLIENT_SHA256','HYE_COMPILER_CLIENT_RELEASE']}, capture_output=True, text=True, timeout=30)
    if result.returncode != 0 or '1 passed; 0 failed' not in result.stdout:
        print(result.stdout, end='', flush=True)
        print(result.stderr, file=sys.stderr, flush=True)
        raise RuntimeError('Static compiler parent fixture did not execute its expected test')
    print(result.stdout, end='', flush=True)
    parent_sha = hashlib.sha256(parent.read_bytes()).hexdigest()
    if installed:
        subprocess.run(['sudo', '--', '/usr/bin/python3', '-I', '-S', str(ROOT/'scripts/compiler_install_probe.py'), '--worker', str(image), '--worker-sha256', image_sha, '--parent', str(parent), '--parent-sha256', parent_sha, '--release', release, '--uid', str(os.getuid()), '--gid', str(os.getgid())], check=True, cwd=ROOT, timeout=45)
    inputs = ['crates/editor-app/src/compiler_client.rs', 'crates/editor-app/tests/compiler_client.rs', 'crates/editor-lua55/src/protocol.rs', 'crates/editor-preview/src/image.rs', 'scripts/compiler_install_probe.py', 'Cargo.toml', 'Cargo.lock', 'data/graph-model.json']
    print(f'Lua {release}: static worker process suite and actual sealed-image parent compilation/conversion/correlation/denial checks passed', flush=True)
    return {'source_inputs': {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in inputs}, 'host':host, 'image_sha256':image_sha, 'image_bytes':image.stat().st_size, 'parent_test_sha256':parent_sha, 'elf_headers_sha256':hashlib.sha256(headers.encode()).hexdigest(), 'interpreter_present':False, 'needed_shared_libraries':[], 'installed_root_package_verified':installed, 'gui_selection_verified':False, 'source_evaluator_verified':False}


def check_worker(root, release, compiler, static=False, installed=False):
    files = [p for p in (ROOT / 'crates/editor-lua55').rglob('*') if p.is_file()]
    files += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'crates/editor-app/src/process_policy.rs']]
    identities = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    target = root / 'worker-target'
    env = dict(os.environ, PKG_CONFIG_LIBDIR=str(root), PKG_CONFIG_PATH='', CARGO_TARGET_DIR=str(target), HYE_LUA55_TEST_RELEASE=release, CXX=compiler)
    flags = ['--locked', '-p', 'hyprland-editor-lua55', '--features', 'native55']
    subprocess.run(['cargo', 'test', *flags, '--', '--test-threads=2'], cwd=ROOT, env=env, check=True, timeout=180)
    subprocess.run(['cargo', 'clippy', *flags, '--all-targets', '--', '-D', 'warnings'], cwd=ROOT, env=env, check=True, timeout=120)
    subprocess.run(['cargo', 'build', *flags, '--release'], cwd=ROOT, env=env, check=True, timeout=180)
    image = target / 'release/hyprland-editor-lua55'
    dynamic = subprocess.check_output(['readelf', '-d', str(image)], text=True, timeout=5)
    needed = re.findall(r'Shared library: \[([^\]]+)\]', dynamic)
    if any(any(part in name.lower() for part in ['lua', 'gtk', 'adwaita', 're2']) for name in needed):
        raise RuntimeError('Private worker unexpectedly links a Lua/GUI/RE2 shared image')
    request = {'protocol': 'hye-lua55-compiler-1', 'nonce': [0] * 32, 'expected_release': release, 'operation': {'kind': 'profile'}}
    wire = json.dumps(request, separators=(',', ':')).encode()
    result = subprocess.run([str(image), '--compiler-worker'], input=wire, capture_output=True, check=True, timeout=6, env={})
    if len(result.stdout) > 16384:
        raise RuntimeError('Private worker profile exceeds its bound')
    profile = json.loads(result.stdout)
    if profile['protocol'] != request['protocol'] or profile['nonce'] != request['nonce'] or profile['request_sha256'] != hashlib.sha256(wire).hexdigest() or profile['profile']['release'] != release or profile['profile']['family'] != 505 or profile['outcome'] != {'kind': 'profile'}:
        raise RuntimeError('Private worker profile/correlation differs')
    if identities != {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}:
        raise RuntimeError('Worker sources changed during verification')
    print(f'Lua {release}: separate compiler worker tests, strict lint and optimized image passed; temporary build removed on scope exit', flush=True)
    result = {'source_inputs': identities, 'image_sha256': hashlib.sha256(image.read_bytes()).hexdigest(), 'shared_dependencies': needed, 'profile_reply': profile, 'gui_selection_verified': False, 'source_evaluator_verified': False}
    if static:
        result['static_parent'] = check_static_parent(root, release, compiler, installed)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', choices=sorted(RELEASES), action='append')
    parser.add_argument('--observation', type=Path)
    parser.add_argument('--compiler', default='c++')
    parser.add_argument('--cpp26-standard', choices=['c++26', 'c++2c'], default='c++2c')
    parser.add_argument('--source-conformance', action='store_true')
    parser.add_argument('--worker-conformance', action='store_true')
    parser.add_argument('--static-conformance', action='store_true')
    parser.add_argument('--installed-conformance', action='store_true', help='Run root-owned temporary package fixtures in the disposable CI job; changes no system package')
    parser.add_argument('--worker-only', action='store_true', help='Check only the separate worker after a worker change; do not claim reference-matrix execution')
    args = parser.parse_args()
    if args.worker_only and (not args.worker_conformance or args.source_conformance):
        parser.error("--worker-only requires --worker-conformance and cannot claim --source-conformance")
    if args.static_conformance and not args.worker_conformance:
        parser.error('--static-conformance requires --worker-conformance')
    if args.installed_conformance and not args.static_conformance:
        parser.error('--installed-conformance requires --static-conformance')
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
            observation = {'release': release, 'archive_url': url, 'archive_sha256': RELEASES[release], 'library_sha256': library}
            if not args.worker_only:
                observed = root / 'observation.json'
                subprocess.run([sys.executable, '-B', str(ROOT / 'scripts/workspace_runtime_probe.py'), '--check', '--compiler', args.compiler, '--cpp26-standard', args.cpp26_standard, '--observation', str(observed)], env=dict(os.environ, PKG_CONFIG_LIBDIR=str(root), PKG_CONFIG_PATH=''), check=True, cwd=ROOT, timeout=300)
                value = json.loads(observed.read_text())
                for profile in value['profiles']:
                    if profile['lua_family'] == '5.5':
                        major, minor, patch = map(int, release.split('.'))
                        if profile['header_release'] != major * 10000 + minor * 100 + patch:
                            raise RuntimeError('Target reference did not use the requested private release')
                observation['matrix'] = value
            if args.source_conformance:
                source_observation = root / 'source-observation.json'
                subprocess.run([sys.executable, '-B', str(ROOT / 'scripts/lua55_source_probe.py'), '--release', release, '--compiler', args.compiler, '--cpp26-standard', args.cpp26_standard, '--observation', str(source_observation)], env=dict(os.environ, PKG_CONFIG_LIBDIR=str(root), PKG_CONFIG_PATH=''), check=True, cwd=ROOT, timeout=360)
                observation['source_reference'] = json.loads(source_observation.read_text())
            if args.worker_conformance:
                observation['compiler_worker'] = check_worker(root, release, args.compiler, args.static_conformance, args.installed_conformance)
            observations.append(observation)
            print(f'Lua {release}: requested private checks passed; temporary source/builds removed on scope exit', flush=True)
    content = {'format': 'hye-workspace-target-release-observation-1', 'profiles': observations, 'selected_compositor_observed': False}
    if args.observation:
        args.observation.write_text(json.dumps(content, indent=2) + '\n')
    print(f'Verified {len(observations)} private target releases; sources/builds removed; application worker compatibility remains unfinished')


if __name__ == '__main__':
    main()
