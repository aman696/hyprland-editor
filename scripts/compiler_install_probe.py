#!/usr/bin/env python3
"""Ephemeral CI package-admission fixture. Never installs a host package.

Run only in the disposable CI job as root, with explicit built-image digests.
All package files are private temporary copies; the Rust test runs after groups,
GID and UID are dropped. No controller, policy, desktop or system file changes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile


def verified(path, digest, bound=64 * 1024 * 1024):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= bound:
            raise RuntimeError('Fixture input is not a bounded regular image')
        data = stream.read(bound + 1)
        after = os.fstat(stream.fileno())
        if len(data) != before.st_size or (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns) or not data.startswith(b'\x7fELF') or hashlib.sha256(data).hexdigest() != digest:
            raise RuntimeError('Fixture image identity differs')
        return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', required=True, type=Path)
    parser.add_argument('--worker-sha256', required=True)
    parser.add_argument('--parent', required=True, type=Path)
    parser.add_argument('--parent-sha256', required=True)
    parser.add_argument('--release', required=True, choices=['5.5.0', '5.5.1'])
    parser.add_argument('--uid', required=True, type=int)
    parser.add_argument('--gid', required=True, type=int)
    args = parser.parse_args()
    if os.geteuid() != 0 or args.uid <= 0 or args.gid <= 0:
        raise RuntimeError('This fixture requires root setup and an explicit ordinary test user')
    worker = verified(args.worker, args.worker_sha256)
    parent = verified(args.parent, args.parent_sha256, 256 * 1024 * 1024)
    with tempfile.TemporaryDirectory(prefix='hye-compiler-package-') as directory:
        root = Path(directory)
        root.chmod(0o755)
        test = root / 'parent-test'
        test.write_bytes(parent)
        test.chmod(0o555)
        manifest = {'format': 'hye-installed-compiler55-1', 'release': args.release, 'image_bytes': len(worker), 'image_sha256': args.worker_sha256}
        for name in ['good', 'writable-directory', 'writable-manifest', 'foreign-image', 'symlink-image', 'symlink-manifest', 'fifo-image', 'hardlink-image', 'wrong-hash', 'unknown-manifest']:
            folder = root / name
            folder.mkdir(mode=0o755)
            registry = folder / 'manifest.json'
            registry.write_text(json.dumps(manifest))
            registry.chmod(0o444)
            image = folder / 'compiler'
            image.write_bytes(worker)
            image.chmod(0o555)
            if name == 'writable-directory':
                folder.chmod(0o777)
            elif name == 'writable-manifest':
                registry.chmod(0o666)
            elif name == 'foreign-image':
                os.chown(image, args.uid, args.gid)
            elif name == 'symlink-image':
                image.unlink()
                image.symlink_to(root / 'good/compiler')
            elif name == 'symlink-manifest':
                registry.unlink()
                registry.symlink_to(root / 'good/manifest.json')
            elif name == 'fifo-image':
                image.unlink()
                os.mkfifo(image, 0o444)
            elif name == 'hardlink-image':
                os.link(image, folder / 'alias')
            elif name == 'wrong-hash':
                registry.chmod(0o644)
                registry.write_text(json.dumps(dict(manifest, image_sha256='0' * 64)))
                registry.chmod(0o444)
            elif name == 'unknown-manifest':
                registry.chmod(0o644)
                registry.write_text(json.dumps(dict(manifest, executable='/unapproved/path')))
                registry.chmod(0o444)
        def drop_privileges():
            os.setgroups([])
            os.setgid(args.gid)
            os.setuid(args.uid)
        result = subprocess.run([str(test), '--ignored', '--exact', 'actual_root_package_registry_admits_only_immutable_static_registered_worker', '--test-threads=1'], env={'HYE_COMPILER_REGISTRY_FIXTURE': str(root)}, preexec_fn=drop_privileges, capture_output=True, text=True, timeout=30)
        print(result.stdout, end='', flush=True)
        if result.returncode != 0 or '1 passed; 0 failed' not in result.stdout:
            print(result.stderr, file=sys.stderr, flush=True)
            raise RuntimeError('Root-package fixture did not execute its expected Rust test')
        print('Root-owned private package admission, nine exact refusals, ordinary-user worker/profile/denial checks and temporary cleanup passed', flush=True)


if __name__ == '__main__':
    main()
