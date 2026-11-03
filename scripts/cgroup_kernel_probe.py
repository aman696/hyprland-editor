#!/usr/bin/env python3
"""Root setup for the ephemeral CI VM only; never auto-configure a desktop.

Creates one private hierarchy beneath an already enabled cgroup v2 root, drops
all privilege before the trusted Rust fixture, and removes its own hierarchy.
No ancestor controller/limit changes, service units or compositor execution.
"""
import argparse
import array
import ctypes
import socket
import struct
import os
from pathlib import Path
import secrets
import stat
import subprocess
import tempfile
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


NS_TEST = 'channel::namespace_tests::pidfs_frame_and_receipt_cross_a_private_pid_namespace'
USERNS_TEST = 'channel::namespace_tests::pidfs_frame_and_receipt_cross_mapped_user_and_pid_namespaces'
RUNTIME_TEST = 'image::probe::sealed_runtime_executes_only_inside_private_root_and_namespaces'
LAUNCH_TEST = 'cgroup::launch::tests::gated_constructor_stays_private_until_verified_release_and_cleans_up'
GATED_RUNTIME_TEST = 'image::probe::gated_runtime_image_keeps_resource_placement_and_private_boundaries_together'


def dropped_command(args, binary, environment, test):
    return ['/usr/bin/setpriv', '--reuid', str(args.uid), '--regid', str(args.gid),
            '--clear-groups', '--bounding-set=-all', '--inh-caps=-all',
            '--ambient-caps=-all', '--no-new-privs', '--', '/usr/bin/env', '-i',
            'LC_ALL=C', *[f'{key}={value}' for key, value in environment.items()],
            str(binary), '--ignored', '--exact', test, '--nocapture', '--test-threads=1']


def packet(kind, serial):
    return b'HYEIPC01' + bytes([kind]) + bytes(7) + struct.pack('<Q', serial)


def bootstrap_frame(channel, expected_pid, uid, gid):
    data, ancillary, flags, _ = channel.recvmsg(24, socket.CMSG_SPACE(16) + socket.CMSG_SPACE(12), socket.MSG_CMSG_CLOEXEC)
    rights, credentials, bad = [], [], False
    try:
        for level, kind, value in ancillary:
            if level == socket.SOL_SOCKET and kind in [socket.SCM_RIGHTS, 4]:
                values = array.array('i')
                values.frombytes(value[:len(value) - len(value) % values.itemsize])
                rights.extend(fd for fd in values if fd >= 0)
                bad |= kind != socket.SCM_RIGHTS or len(value) % values.itemsize != 0 or any(fd < 0 for fd in values)
            elif level == socket.SOL_SOCKET and kind == socket.SCM_CREDENTIALS and len(value) == 12:
                credentials.append(struct.unpack('iII', value))
            else:
                bad = True
        if (flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC) or bad or data != packet(1, 1)
                or len(rights) != 1 or credentials != [(expected_pid, uid, gid)]):
            raise RuntimeError('Invalid private GUI bootstrap envelope')
        return rights.pop()
    finally:
        for fd in rights:
            os.close(fd)


def namespace_channel(args, binary, driver, mapped=False):
    # Root creates only its own child's PID namespace. The current namespace
    # stays unchanged, and all repository test roles execute after setpriv.
    libc = ctypes.CDLL(None, use_errno=True)
    libc.unshare.argtypes = [ctypes.c_int]
    libc.setns.argtypes = [ctypes.c_int, ctypes.c_int]
    clone_newpid = 0x20000000
    original = os.open('/proc/self/ns/pid_for_children', os.O_RDONLY | os.O_CLOEXEC)
    before = os.stat('/proc/self/ns/pid').st_ino
    user_before = os.stat('/proc/self/ns/user').st_ino
    mount_before = os.stat('/proc/self/ns/mnt').st_ino
    test = USERNS_TEST if mapped else NS_TEST
    left, right = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC | socket.SOCK_NONBLOCK)
    parent, gui_bootstrap = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC | socket.SOCK_NONBLOCK)
    for endpoint in [left, right, parent, gui_bootstrap]:
        endpoint.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
    parent.settimeout(10)
    gui = broker = None
    frame = gui_pin = broker_pin = None
    def place():
        write(driver / 'cgroup.procs', '0')
    try:
        command = dropped_command(args, binary, {'HYE_PIDNS_CHANNEL_ROLE': 'gui', 'HYE_PIDNS_DATA_FD': left.fileno(), 'HYE_PIDNS_BOOTSTRAP_FD': gui_bootstrap.fileno()}, test)
        gui = subprocess.Popen(command, pass_fds=(left.fileno(), gui_bootstrap.fileno()), preexec_fn=place,
                               env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C'}, stdin=subprocess.DEVNULL)
        left.close(); gui_bootstrap.close()
        frame = bootstrap_frame(parent, gui.pid, args.uid, args.gid)
        gui_pin = os.pidfd_open(gui.pid)
        if libc.unshare(clone_newpid):
            raise OSError(ctypes.get_errno(), 'Cannot create the private CI PID namespace')
        try:
            environment = {'HYE_PIDNS_CHANNEL_ROLE': 'broker', 'HYE_PIDNS_DATA_FD': right.fileno(), 'HYE_PIDNS_ANCHOR_FD': gui_pin, 'HYE_PIDNS_FRAME_FD': frame}
            if mapped:
                environment.update(HYE_PIDNS_OUTER_UID=args.uid, HYE_PIDNS_OUTER_GID=args.gid,
                                   HYE_PIDNS_OUTER_USER_NS=user_before)
                # Only fixed system setup tools retain privilege. First make a
                # private mount/proc for this already-private PID1: id-map tools
                # use proc paths and must never resolve its PID1 to host init.
                # The fixed mount constructor alone receives SYS_ADMIN in the
                # initial namespace. It runs as the future outer fixture user;
                # self-only UID/GID maps need no subordinate-ID edits/helpers.
                # No repository executable runs until the inner setpriv has
                # dropped every capability, bounding bit and supplementary group.
                command = ['/usr/bin/setpriv', '--reuid', str(args.uid), '--regid', str(args.gid),
                           '--clear-groups', '--bounding-set=-all,+sys_admin',
                           '--inh-caps=-all,+sys_admin', '--ambient-caps=-all,+sys_admin', '--no-new-privs', '--',
                           '/usr/bin/unshare', '--mount', '--propagation', 'private', '--mount-proc', '--',
                           '/usr/bin/unshare', '--user', '--map-user=1000', '--map-group=1000', '--keep-caps', '--',
                           '/usr/bin/setpriv', '--reuid=1000', '--regid=1000', '--keep-groups',
                           '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all', '--no-new-privs',
                           '--', '/usr/bin/env', '-i', 'LC_ALL=C',
                           *[f'{key}={value}' for key, value in environment.items()],
                           str(binary), '--ignored', '--exact', test, '--nocapture', '--test-threads=1']
            else:
                command = dropped_command(args, binary, environment, test)
            broker = subprocess.Popen(command, pass_fds=(right.fileno(), gui_pin, frame), preexec_fn=place,
                                      env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C'}, stdin=subprocess.DEVNULL)
        finally:
            if libc.setns(original, clone_newpid):
                raise OSError(ctypes.get_errno(), 'Cannot restore CI future-child namespace')
        right.close()
        broker_pin = os.pidfd_open(broker.pid)
        if parent.sendmsg([packet(1, 1)], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [broker_pin]))], socket.MSG_NOSIGNAL) != 24:
            raise RuntimeError('Partial private process-anchor transfer')
        if broker.wait(timeout=12):
            raise RuntimeError('Namespace broker did not verify the receipt')
        if parent.send(packet(2, 2), socket.MSG_NOSIGNAL) != 24:
            raise RuntimeError('Partial namespace completion receipt')
        if gui.wait(timeout=12):
            raise RuntimeError('GUI role did not verify exact pixels and teardown')
        if os.stat('/proc/self/ns/pid').st_ino != before:
            raise RuntimeError('CI setup current PID namespace changed')
        if os.stat('/proc/self/ns/user').st_ino != user_before or os.stat('/proc/self/ns/mnt').st_ino != mount_before:
            raise RuntimeError('CI setup user/mount namespace changed')
        print('Private PID namespace channel passed; current setup namespace unchanged', flush=True)
        if mapped:
            print('HYE_USERNS_PROVIDER mapped channel passed; setup user/mount/PID namespaces unchanged', flush=True)
    finally:
        for child in [broker, gui]:
            if child is not None and child.poll() is None:
                child.kill(); child.wait(timeout=3)
        for endpoint in [left, right, parent, gui_bootstrap]:
            endpoint.close()
        for fd in [frame, gui_pin, broker_pin, original]:
            if fd is not None:
                os.close(fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--uid', type=int, required=True)
    parser.add_argument('--gid', type=int, required=True)
    parser.add_argument('--test-executable', type=Path, required=True)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--namespace-channel', action='store_true')
    modes.add_argument('--user-namespace-channel', action='store_true')
    modes.add_argument('--runtime-image-probe', action='store_true')
    modes.add_argument('--gated-launch', action='store_true')
    modes.add_argument('--gated-runtime-image', action='store_true')
    parser.add_argument('--probe-binary', type=Path)
    parser.add_argument('--constructor-binary', type=Path)
    parser.add_argument('--expect-runtime-unavailable', action='store_true')
    args = parser.parse_args()
    if os.geteuid() != 0 or args.uid <= 0 or args.gid <= 0:
        raise SystemExit('Use root only for CI setup, with a non-root fixture UID/GID')
    binary = args.test_executable.resolve(strict=True)
    if not binary.is_file():
        raise SystemExit('An already compiled trusted test executable is required')
    probe = constructor = None
    if args.gated_launch:
        if args.probe_binary is None or args.constructor_binary is not None or args.expect_runtime_unavailable:
            raise SystemExit('The launch gate test requires only its trusted static probe binary')
        probe = args.probe_binary.resolve(strict=True)
        if not probe.is_file():
            raise SystemExit('Missing trusted static launch probe')
    elif args.runtime_image_probe or args.gated_runtime_image:
        if args.probe_binary is None:
            raise SystemExit('The runtime test requires the already compiled trusted static probe')
        probe = args.probe_binary.resolve(strict=True)
        if not probe.is_file():
            raise SystemExit('Missing trusted static probe')
        constructor = (args.constructor_binary or Path('/usr/bin/bwrap')).resolve(strict=True)
        if not constructor.is_file():
            raise SystemExit('Missing trusted namespace constructor')
    elif args.probe_binary is not None or args.constructor_binary is not None or args.expect_runtime_unavailable:
        raise SystemExit('Runtime arguments are accepted only for the runtime-image test')
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
        if args.namespace_channel or args.user_namespace_channel:
            print('Kernel observation:', os.uname().release, flush=True)
            namespace_channel(args, binary, driver, mapped=args.user_namespace_channel)
            return
        # Canary is readable by the trusted driver before sandbox construction,
        # but its random host pathname must be absent inside the private root.
        with tempfile.TemporaryDirectory(prefix='hye-ci-runtime-') as temporary:
            canary = Path(temporary) / 'host-secret'
            canary.write_text('synthetic host secret; no real credentials\n')
            canary.chmod(0o600)
            os.chown(canary, args.uid, args.gid)
            os.chown(temporary, args.uid, args.gid)
            environment = ({'HYE_RUNTIME_PROBE_BIN': probe, 'HYE_RUNTIME_BWRAP_BIN': constructor,
                            'HYE_RUNTIME_EXPECT_UNAVAILABLE': int(args.expect_runtime_unavailable),
                            'HYE_RUNTIME_PROBE_HOST_CANARY': canary}
                           if args.runtime_image_probe or args.gated_runtime_image else {'HYE_CGROUP_TEST_ROOT': path})
            if args.gated_launch:
                environment['HYE_LAUNCH_TEST_EXECUTABLE'] = probe
            if args.gated_runtime_image:
                environment['HYE_CGROUP_TEST_ROOT'] = path
            if args.gated_launch:
                test = LAUNCH_TEST
            elif args.gated_runtime_image:
                test = GATED_RUNTIME_TEST
            elif args.runtime_image_probe:
                test = RUNTIME_TEST
            else:
                test = TEST
            command = dropped_command(args, binary, environment, test)
            # This single-threaded CI setup process places only its own child
            # in the bounded driver leaf, before setpriv and repository code.
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
