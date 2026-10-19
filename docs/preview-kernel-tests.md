# Opt-in cgroup kernel verification

The `cgroup-kernel` CI job prepares a fresh dedicated hierarchy on an ephemeral Ubuntu runner. It exercises the actual resource-domain library without modifying the developer's desktop, selecting a desktop delegation, starting a host service or running a compositor. The fixture is compiled locally but positive kernel execution is **unproven until an exact hosted run succeeds**.

The root setup script is `scripts/cgroup_kernel_probe.py`. It refuses a non-cgroup-v2 mount, an unsuitable root or missing already-enabled CPU/memory/process controllers. It never enables an ancestor's controllers or changes ancestor limits. It creates one unpredictable private directory, limits the whole fixture to 1 GiB/256 tasks/two CPU cores, and creates a bounded driver leaf. Setup then delegates only that new private root and drops UID/GID, supplementary groups and capabilities before executing the compiled test. No root helper is installed in the application.

The Rust test is explicitly ignored in ordinary runs and requires this provider. It verifies non-root matching credentials, empty permitted/effective/bounding/ambient capabilities and no-new-privileges. Each trusted worker places itself in the intended leaf before exec, receives a live resource-directory descriptor, verifies actual limits and that it is the sole process there, and waits for the driver's membership check before starting a load. Environment text alone cannot create that descriptor. This is trusted fixture placement, not production stopped-child/compositor launch certification.

The default leaf profile is 512 MiB memory, no swap, group OOM handling, 64 tasks and one aggregate CPU core. Three scenarios test:

- Four runnable CPU workers with positive kernel usage and throttling counters; this fixture requires at least two available CPUs.
- Committed anonymous allocations with a kernel leaf OOM event and SIGKILL, rather than relying on a process address-space limit.
- Child creation until the task controller reports refusal, followed by whole-subtree kill with the descendants still present.

Every scenario uses the library's explicit cleanup and requires kernel absence of the removed leaf. Bounded result records report the exact counters and profile. The setup process stays outside the private hierarchy and finally kills remaining private tasks, waits for emptiness and removes only its own identified hierarchy. Failed cleanup is a failed job. Missing platform prerequisites fail explicitly, not as a passing security test.

This demonstrates only the exercised kernel mechanisms if the run passes. It does not certify hostile concurrent namespace/path substitution, production membership before untrusted execution, a verified runtime image/syscall/device policy, malicious descriptor flooding, GUI-loss/crash supervision, frame/input provenance or whole-lab release acceptance. Memory-controller accounting is distinct from GUI process-tree PSS. The [full preview scope](preview-supervision.md), [channel limits](preview-channel.md) and every original blueprint gate remain mandatory.
