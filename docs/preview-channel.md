# Private frame channel

`crates/editor-preview/src/channel.rs` connects the sealed raw-frame producer and receiver through an unnamed Unix sequenced-packet socket pair. It transfers synthetic frames between real processes. It does not yet connect to a compositor, negotiate Wayland protocols, route input or establish a certified sandbox profile.

## Endpoint and process ownership

The factory creates close-on-exec, nonblocking endpoints. An explicitly inherited endpoint must be an unnamed, connected local sequenced-packet socket; ordinary files, stream sockets and named endpoints are refused. The API never connects to a pathname or a host display. It configures sender credentials and a small send buffer, checks the resulting settings, and exposes borrowed socket/process descriptors for future event-loop integration.

Each bound endpoint owns a live pidfd. Binding supports this process, a still-running direct child, or a worker's explicitly expected and still-visible direct parent. Reaping, termination and reparenting during parent binding cause refusal. Real and effective user/group IDs must match. The selected peer must send matching kernel-checked per-message PID/user/group credentials; checks of the live process pin bracket receiving a message. Creation-time socket credentials are insufficient after descriptor inheritance, so they are not used to authenticate individual messages.

This identifies a live process under the supported credential/PID-visibility assumptions. It does not identify its executable, projection, namespace or resource profile. A process can exit immediately after a successful observation; callers must monitor the exposed process descriptor and stop on exit. The launcher must still close unwanted descriptor copies and establish executable, namespace, credential/capability and resource conditions. Different PID namespace mappings are not certified by the ordinary child fixture.

## Record and credit protocol

Each packet has exactly 24 bytes: eight-byte `HYEIPC01` magic/version, one-byte role, seven zero reserved bytes and an eight-byte nonzero little-endian transport serial. Role 1 carries one frame descriptor. Roles 2 and 3 acknowledge raw-frame acceptance or rate dropping, respectively, and carry no descriptor. Transport serials are sequential and receipts must match the sole pending frame. They are distinct from the frame record's session, projection generation and sequence checks.

The cooperative sender permits one frame in flight and consumes its input descriptor on all outcomes. It cannot send another before a matching receipt. The receiver processes one packet per call; a pending receipt must flush before another packet is read. It accepts the descriptor through the sealed-frame validator, acknowledges a rate drop without creating an extra pixel buffer, and retains at most one unsent receipt. Operations use nonblocking send/receive and do not contain an unbounded packet-drain loop.

Inherited endpoints disable unsolicited process-descriptor and security-label records. A defensive parser also closes an unexpected known PIDFD record before refusal; it does not mistake such a record for opaque metadata.

Fixed ancillary storage receives kernel credentials and rights. All delivered rights are collected under owned handles before validation. Wrong roles, unknown/reserved fields, serial mismatch, missing/extra descriptors, missing/wrong credentials, truncated packets and invalid raw frames fail closed. Fatal errors shut down the channel; the receiver owns and clears pixel buffers on those errors or explicit stop. Dropping a bound endpoint shuts it down as well. No receipt means compositor readiness or actual output provenance.

## Resource limits and unproved boundaries

The single-credit protocol bounds the normal sender's in-flight frame and the receiver's pending receipt. The configured small kernel send buffer supplements it. It is not a hard two-descriptor kernel queue quota against a compromised peer bypassing this library. Unix receive-buffer settings do not supply that guarantee, and this implementation does not rely on them. Actual kernel memory/task/CPU enforcement and malicious-peer flooding/descriptor-exhaustion tests remain mandatory before a lab profile can execute.

The channel owns its received pixel buffers, but does not own a compositor process tree, rendering resources or private filesystem. Socket shutdown is not whole-lab teardown. Input release, GUI-loss supervision, resource violations, timeouts, crash escalation, runtime-image validation and namespace/device/syscall policies remain required. No untrusted config was executed and no host compositor setting changed during these tests.

## Verification

Fourteen channel checks cover a real child sender and receipt, kernel credentials versus creation-time socket identity, dead process pins on both roles, poll readiness, nonblocking/close-on-exec properties, endpoint refusal, explicit stop/disconnect/invalid-frame pixel release, 192 single-bit packet-header mutations, invalid receipts and missing/extra/truncated descriptors. Pipe EOF proves that rejected or queued writer descriptors close, including a saturated-receipt case. That case also verifies that backpressure prevents another frame read until the receipt can flush.

An isolated child also uses a 64-descriptor soft limit and 256 unexpected PIDFD-record refusals, checking that its descriptor count does not grow. Kernels without that socket option exercise the exact unsupported-option path instead; this is reported separately and does not establish positive PIDFD-record behavior. Pipe EOF uses a three-second bound because other parallel tests can briefly inherit close-on-exec writers between fork and exec.

These cases use ordinary same-user child processes and synthetic pixels. They are not a compositor, cross-namespace, hostile-peer containment or full E09/S-gate certificate. The earlier [frame](preview-frames.md) and [resource](preview-supervision.md) limits still apply.

Interface references: the Linux man-pages project's [Unix socket manual](https://man7.org/linux/man-pages/man7/unix.7.html) and [pidfd_open manual](https://man7.org/linux/man-pages/man2/pidfd_open.2.html). Profile behavior must also be verified against the supported kernel and launcher.

The extra descriptor record is defined in the kernel's [v6.12 socket header](https://github.com/torvalds/linux/blob/v6.12/include/linux/socket.h); allocation/receive handling is in [the corresponding SCM implementation](https://github.com/torvalds/linux/blob/v6.12/include/net/scm.h). Supported runtime/kernel profiles must enumerate ancillary behavior; future kernel extensions are not automatically certified.
