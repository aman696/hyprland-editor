#!/usr/bin/env python3
"""Root setup for the ephemeral CI VM only; never auto-configure a desktop.

Creates one private hierarchy beneath an already enabled cgroup v2 root, drops
all privilege before the trusted Rust fixture, and removes its own hierarchy.
No ancestor controller/limit changes, service units or compositor execution.
"""
import argparse
import os
from pathlib import Path
import secrets
import stat
import subprocess
import time

ROOT = Path('/sys/fs/cgroup')
REQUIRED = {'cpu', 'memory', 'pids'}
TEST = 'cgroup::kernel_tests::delegated_kernel_limits_enforce_and_teardown_whole_subtrees'


def read(path):
    with path.open('rb') as file:
        value = file.read(4097)
    if len(value) > 4096:
        raise RuntimeError('Kernel control reply exceeds its bound')
    return value.decode('ascii').strip()


def write(path, value):
    descriptor = os.open(path, os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        data = value.encode('ascii')
        if os.write(descriptor, data) != len(data):
            raise RuntimeError('Partial private cgroup control write')
    finally:
        os.close(descriptor)


def limit(path, name, value):
    write(path / name, value)
    if read(path / name) != value:
        raise RuntimeError(f'Private {name} limit readback differs')


def populated(path):
    values = [row.split() for row in read(path / 'cgroup.events').splitlines()
              if row.split()[:1] == ['populated']]
    if len(values) != 1 or values[0] not in [['populated', '0'], ['populated', '1']]:
        raise RuntimeError('Missing or ambiguous private population event')
    return values[0][1] == '1'


def remove_children(path, depth=0, count=None):
    if count is None:
        count = [0]
    if depth > 8:
        raise RuntimeError('Private cleanup nesting exceeds its bound')
    for entry in list(path.iterdir()):
        info = entry.lstat()
        if stat.S_ISDIR(info.st_mode):
            count[0] += 1
            if count[0] > 256:
                raise RuntimeError('Private cleanup node count exceeds its bound')
            remove_children(entry, depth + 1, count)
            entry.rmdir()
        elif stat.S_ISLNK(info.st_mode):
            raise RuntimeError('Unexpected link in private cgroup hierarchy')


def cleanup(path, descriptor):
    current = path.lstat()
    held = os.fstat(descriptor)
    if (current.st_dev, current.st_ino) != (held.st_dev, held.st_ino):
        raise RuntimeError('CI resource root identity changed; refusing pathname cleanup')
    write(path / 'cgroup.kill', '1')
    deadline = time.monotonic() + 10
    while populated(path):
        if time.monotonic() >= deadline:
            raise RuntimeError('Private CI subtree did not become empty')
        time.sleep(.02)
    remove_children(path)
    path.rmdir()
    print('Private CI hierarchy empty and removed', flush=True)


def metadata_prerequisites(path):
    """Read-only shape checks, not filesystem/capability validation."""
    try:
        kind = read(path / 'cgroup.type')
    except FileNotFoundError:
        # Linux omits cgroup.type on the global root (CFTYPE_NOT_ON_ROOT).
        kind = None
    if kind not in [None, 'domain']:
        raise RuntimeError('A threaded or invalid cgroup parent is unsupported')
    for control in ['cgroup.controllers', 'cgroup.subtree_control']:
        if not REQUIRED.issubset(set(read(path / control).split())):
            raise RuntimeError('Required root controllers are not already enabled; no ancestor changes made')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--uid', type=int, required=True)
    parser.add_argument('--gid', type=int, required=True)
    parser.add_argument('--test-executable', type=Path, required=True)
    args = parser.parse_args()
    if os.geteuid() != 0 or args.uid <= 0 or args.gid <= 0:
        raise SystemExit('Use root only for CI setup, with a non-root fixture UID/GID')
    binary = args.test_executable.resolve(strict=True)
    if not binary.is_file():
        raise SystemExit('An already compiled trusted test executable is required')
    fs = subprocess.run(['/usr/bin/stat', '-f', '-c', '%T', str(ROOT)],
                        capture_output=True, text=True, timeout=3, check=True).stdout.strip()
    if fs != 'cgroup2fs':
        raise SystemExit('The ephemeral VM requires a cgroup v2 root')
    metadata_prerequisites(ROOT)
    path = ROOT / ('hye-ci-' + secrets.token_hex(16))
    path.mkdir(mode=0o700)
    descriptor = None
    child = None
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        if read(path / 'cgroup.type') != 'domain':
            raise RuntimeError('The fresh private hierarchy is not a normal domain')
        # Only the freshly owned private root is configured. Ancestor limits and
        # subtree settings remain untouched. Bound the complete fixture as well.
        limit(path, 'memory.max', str(1024 * 1024 * 1024))
        limit(path, 'memory.swap.max', '0')
        limit(path, 'pids.max', '256')
        limit(path, 'cpu.max', '200000 100000')
        write(path / 'cgroup.subtree_control', '+cpu +memory +pids')
        if not REQUIRED.issubset(set(read(path / 'cgroup.subtree_control').split())):
            raise RuntimeError('Private controller enablement did not match')
        driver = path / 'driver'
        driver.mkdir(mode=0o700)
        limit(driver, 'memory.max', str(256 * 1024 * 1024))
        limit(driver, 'memory.swap.max', '0')
        limit(driver, 'pids.max', '64')
        limit(driver, 'cpu.max', '200000 100000')
        # Kernel delegation: ownership of the dedicated root and migration /
        # subtree controls, never ownership of an ancestor shared control.
        os.chown(path, args.uid, args.gid)
        for name in ['cgroup.procs', 'cgroup.threads', 'cgroup.subtree_control']:
            os.chown(path / name, args.uid, args.gid)
        command = ['/usr/bin/setpriv', '--reuid', str(args.uid), '--regid', str(args.gid),
                   '--clear-groups', '--bounding-set=-all', '--inh-caps=-all',
                   '--ambient-caps=-all', '--no-new-privs', '--', '/usr/bin/env', '-i',
                   'LC_ALL=C', f'HYE_CGROUP_TEST_ROOT={path}', str(binary), '--ignored',
                   '--exact', TEST, '--nocapture', '--test-threads=1']
        # This single-threaded CI setup process uses preexec only to place its
        # own child in the bounded driver leaf. setpriv drops privilege before
        # executing any repository test code. No caller-selected PID is moved.
        def place_driver():
            write(driver / 'cgroup.procs', '0')
        print('Kernel observation:', os.uname().release, flush=True)
        child = subprocess.Popen(command, preexec_fn=place_driver,
                                 env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C'},
                                 stdin=subprocess.DEVNULL)
        try:
            result = child.wait(timeout=50)
        except subprocess.TimeoutExpired:
            raise RuntimeError('Private CI fixture exceeded its whole-run deadline') from None
        if result:
            raise RuntimeError(f'Private kernel fixture failed with status {result}')
    finally:
        # This setup process is outside its private hierarchy. Even after a
        # cleanup error, terminate/reap the known direct driver before returning.
        try:
            if descriptor is not None:
                try:
                    cleanup(path, descriptor)
                finally:
                    os.close(descriptor)
            else:
                # No opened identity exists; never unlink an unverified path.
                print('Unresolved private root creation:', path, flush=True)
        finally:
            if child is not None and child.poll() is None:
                child.kill()
                child.wait(timeout=3)


if __name__ == '__main__':
    main()
