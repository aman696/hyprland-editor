# Repeatable resource observations

The [resource probe](../scripts/resource_probe.py) launches the supplied binary on a separate Xvfb display and D-Bus session with empty private HOME/XDG folders, Cairo and in-memory settings. It samples summed application-process-tree PSS/RSS and CPU ticks, including helpers spawned from other threads. It records process start identities, timing, binary SHA-256, kernel/architecture and explicit scope limits. The display/bus infrastructure is excluded from application totals. The private process groups and fixture folders are cleaned up afterwards.

The current [raw observation](../data/resource-idle-observation.json) uses release binary SHA-256 `623d7d5fabc12053f9dafb9417f49dddde4752361f9c0eecd192106b85fa9cb5`, built for original-table head `81890547f89b9f873bde5036480b61a13479bdc0`. Each trial has a fresh empty fixture, a two-second startup interval and 60 seconds of observations at approximately half-second intervals. Host pkg-config metadata reports GTK 4.22.5 and libadwaita 1.9.4; that metadata does not independently certify loaded libraries. This is not a cold system-page-cache experiment.

| Trial | Maximum sampled process-tree PSS | Maximum sampled RSS | CPU ticks over 60 seconds |
| --- | ---: | ---: | ---: |
| 1 | 94,482 KiB (92.3 MiB) | 178,848 KiB | 0 |
| 2 | 95,516 KiB (93.3 MiB) | 179,948 KiB | 0 |
| 3 | 94,899 KiB (92.7 MiB) | 179,524 KiB | 0 |

PSS apportions shared pages and is the blueprint's memory metric. RSS is larger because it includes resident shared pages in full. Zero ticks means CPU use was below the resolution of this particular observation, not a promise of zero CPU use. Each recorded empty-idle trial meets the 100 MiB PSS observation threshold and the blueprint's under-0.1%-of-one-core threshold averaged over at least 60 seconds.

**The full ordinary-editor resource gate remains unproven.** These runs contain one empty main workspace, no imported source, open review, undo history, event stream, asset processing or parser helper. A representative configuration and the required import/edit/search/history/worker scenarios still need measurements, along with physical Wayland/GPU/accessibility profiles and longer sessions. Peaks and short-lived processes between samples may be missed. The recorded repository revision is observational; the binary's independent SHA-256 identifies what ran. The prior 0.4 sample above target is retained as historical evidence, and differences in shared-page accounting/profile do not establish a version-to-version improvement.

## Reproduce

Build the release binary, install Xvfb and dbus-run-session through your distribution, then run from the repository root:

```sh
python3 scripts/resource_probe.py --output resource-observation.json --require-thresholds
```

For an extracted test display binary, pass `--xvfb /absolute/path/to/Xvfb`. The default is three trials with 60-second CPU windows. The probe refuses unavailable display/bus tools and has no host-display fallback. `--repeat`, `--settle-seconds`, `--sample-seconds` and `--interval-seconds` accept bounded values. Short observations remain possible, but cannot pass the 60-second CPU window; `--require-thresholds` writes the observation and exits unsuccessfully when an empty-idle threshold/window fails. Passing that flag never certifies other workloads.

Three process integration checks run in CI:

```sh
python3 -B -m unittest discover -s scripts -p test_resource_probe.py
```

They observe a helper launched by a still-running non-main thread, terminate an owned group after its original leader exits, and verify fixture cleanup when the GUI fails to launch. An actual one-second GUI probe also demonstrated that the required CPU window refuses short observations. These checks do not establish preview/apply/recovery isolation or teardown certification. The original blueprint's representative workload, P50/P95 latency, large inputs, event storms, asset/preview budgets and all full resource/release gates remain required.
