#!/usr/bin/env python3
"""Observe an idle release GUI on a private display/bus with empty fixture state.

This is a repeatable empty-workspace observation, not full resource acceptance.
No host compositor/configuration is used. Standard-library-only, Linux /proc.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import tempfile
import time


def identity(pid):
    # comm can contain spaces/parentheses; fields begin after its final ')'.
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return (pid, int(fields[19])), int(fields[11]) + int(fields[12])


def descendants(pid):
    result, pending = set(), [pid]
    while pending:
        current = pending.pop()
        if current in result:
            continue
        result.add(current)
        if len(result) > 256:
            raise RuntimeError("Observed process tree exceeds 256 processes")
        try:
            # Children spawned from any thread belong to this observation too.
            threads = list(Path(f"/proc/{current}/task").iterdir())
            for thread in threads:
                try:
                    pending.extend(map(int, (thread / "children").read_text().split()))
                except FileNotFoundError:
                    pass
        except FileNotFoundError:
            pass
    return result


def sample(pid):
    rows = []
    for child in sorted(descendants(pid)):
        try:
            start, ticks = identity(child)
            values = {}
            for line in Path(f"/proc/{child}/smaps_rollup").read_text().splitlines():
                if ":" in line:
                    key, value = line.split(":", 1)
                    if key in {"Rss", "Pss", "Private_Clean", "Private_Dirty", "SwapPss"}:
                        values[key] = int(value.split()[0])
            if identity(child)[0] != start:
                raise RuntimeError("Observed process identity changed during sampling")
            if not {"Rss", "Pss"}.issubset(values):
                raise RuntimeError("Incomplete smaps_rollup observation")
            rows.append({"pid": child, "start_ticks": start[1], "cpu_ticks": ticks, "kib": values})
        except (FileNotFoundError, ProcessLookupError):
            if child == pid:
                raise RuntimeError("GUI exited during observation") from None
            # A short-lived helper may disappear; its unobserved peak is unknown.
    if not any(row["pid"] == pid for row in rows):
        raise RuntimeError("GUI unavailable during observation")
    return rows


def stop(process):
    if process is None:
        return
    # Kill only the process group explicitly started by this probe, never names.
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=3)


def trial(binary, xvfb, bus, args):
    server = application = None
    with tempfile.TemporaryDirectory(prefix="hye-resource-") as directory:
        root = Path(directory)
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(("GTK_", "GDK_", "GSK_", "HYPRLAND_", "WAYLAND_", "DBUS_"))
               and not key.startswith("XDG_") and key not in {"DISPLAY", "XAUTHORITY", "SESSION_MANAGER", "AT_SPI_BUS_ADDRESS"}}
        for name, variable in [("home", "HOME"), ("config", "XDG_CONFIG_HOME"),
                               ("cache", "XDG_CACHE_HOME"), ("state", "XDG_STATE_HOME"),
                               ("data", "XDG_DATA_HOME"), ("runtime", "XDG_RUNTIME_DIR")]:
            path = root / name
            path.mkdir(mode=0o700)
            env[variable] = str(path)
        env.update(GDK_BACKEND="x11", GSK_RENDERER="cairo", GSETTINGS_BACKEND="memory",
                   GIO_USE_VFS="local", XAUTHORITY=str(root / "empty-authority"))
        with (root / "display.log").open("w") as display_log, (root / "gui.log").open("w") as gui_log:
            read_fd, write_fd = os.pipe()
            try:
                try:
                    server = subprocess.Popen([str(xvfb), "-displayfd", str(write_fd), "-screen", "0",
                                           "1280x900x24", "-nolisten", "tcp", "-extension", "GLX"],
                                              pass_fds=(write_fd,), stdout=display_log, stderr=display_log,
                                              start_new_session=True)
                finally:
                    os.close(write_fd)
                if not select.select([read_fd], [], [], 5)[0]:
                    raise RuntimeError("Private display did not become ready")
                display = os.read(read_fd, 32).decode("ascii").strip()
                if not display.isdecimal() or server.poll() is not None:
                    raise RuntimeError("Private display returned no valid display number")
                env["DISPLAY"] = f":{display}"
                application = subprocess.Popen([str(bus), "--", str(binary)], env=env,
                                               stdout=gui_log, stderr=gui_log, start_new_session=True)
                deadline, gui = time.monotonic() + 10, None
                while gui is None:
                    if application.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError("GUI failed to start on its private bus/display")
                    for pid in descendants(application.pid):
                        try:
                            if Path(f"/proc/{pid}/exe").samefile(binary):
                                gui = identity(pid)[0]
                                break
                        except (FileNotFoundError, ProcessLookupError):
                            pass
                    if gui is None:
                        time.sleep(0.05)
                time.sleep(args.settle_seconds)
                initial = sample(gui[0])
                baseline = {(r["pid"], r["start_ticks"]): r["cpu_ticks"] for r in initial}
                latest = dict(baseline)
                started, observations = time.monotonic(), []
                while True:
                    if identity(gui[0])[0] != gui or application.poll() is not None or server.poll() is not None:
                        raise RuntimeError("Private GUI/display exited or changed identity")
                    rows = sample(gui[0])
                    elapsed = time.monotonic() - started
                    for row in rows:
                        latest[(row["pid"], row["start_ticks"])] = row["cpu_ticks"]
                    observations.append({"elapsed_seconds": elapsed, "processes": rows,
                                         "total_pss_kib": sum(r["kib"]["Pss"] for r in rows),
                                         "total_rss_kib": sum(r["kib"]["Rss"] for r in rows)})
                    if elapsed >= args.sample_seconds:
                        break
                    time.sleep(min(args.interval_seconds, args.sample_seconds - elapsed))
                ticks = sum(latest.values()) - sum(baseline.values())
                cpu = ticks / os.sysconf("SC_CLK_TCK") / elapsed * 100
                maximum = max(o["total_pss_kib"] for o in observations)
                return {"samples": observations, "max_observed_pss_kib": maximum,
                        "max_observed_rss_kib": max(o["total_rss_kib"] for o in observations),
                        "observed_cpu_ticks": ticks, "observed_single_core_cpu_percent": cpu,
                        "empty_idle_thresholds_met": maximum <= 102400 and cpu < 0.1 and elapsed >= 60,
                        "idle_cpu_window_complete": elapsed >= 60}
            except Exception as error:
                gui_log.flush()
                details = (root / "gui.log").read_text(errors="replace")[-2048:]
                raise RuntimeError(f"{error}\n{details}") from error
            finally:
                os.close(read_fd)
                try:
                    stop(application)
                finally:
                    stop(server)


def package_metadata():
    versions = {}
    for package in ["gtk4", "libadwaita-1"]:
        try:
            result = subprocess.run(["pkg-config", "--modversion", package],
                                    capture_output=True, text=True, timeout=3)
            versions[package] = result.stdout.strip()[:128] if result.returncode == 0 else None
        except (FileNotFoundError, subprocess.TimeoutExpired):
            versions[package] = None
    return versions


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/hyprland-editor"))
    parser.add_argument("--xvfb", type=Path, default=shutil.which("Xvfb"))
    parser.add_argument("--bus", type=Path, default=shutil.which("dbus-run-session"))
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--settle-seconds", type=float, default=2)
    parser.add_argument("--sample-seconds", type=float, default=60)
    parser.add_argument("--interval-seconds", type=float, default=0.5)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--require-thresholds", action="store_true", help="Fail unless the empty-idle thresholds and 60-second CPU window pass")
    args = parser.parse_args()
    if not (1 <= args.repeat <= 10 and 0 <= args.settle_seconds <= 10 and
            1 <= args.sample_seconds <= 120 and 0.05 <= args.interval_seconds <= 2):
        parser.error("Probe parameters exceed bounded repeat/timing limits")
    if args.xvfb is None or args.bus is None:
        parser.error("Xvfb and dbus-run-session are required; no host-display fallback")
    binary, xvfb, bus = (p.resolve(strict=True) for p in (args.binary, args.xvfb, args.bus))
    revision = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    results = []
    for index in range(args.repeat):
        results.append(trial(binary, xvfb, bus, args))
        print(f"Completed empty-idle trial {index+1}/{args.repeat}",flush=True)
    if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_hash:
        raise RuntimeError("Binary changed during resource observation")
    result = {"format": "hye-resource-observation-1", "scenario": "idle empty GUI",
              "binary_sha256": binary_hash,
              "repository_revision_observed": revision,
              "pkg_config_versions_observed": package_metadata(),
              "platform": {"kernel": os.uname().release, "machine": os.uname().machine,
                           "clock_ticks_per_second": os.sysconf("SC_CLK_TCK")},
              "profile": {"renderer": "cairo", "display": "private Xvfb 1280x900x24",
                          "bus": "private dbus-run-session", "state": "empty private HOME/XDG folders",
                          "settings": "in memory", "settle_seconds": args.settle_seconds,
                          "sample_seconds": args.sample_seconds, "interval_seconds": args.interval_seconds},
              "empty_idle_thresholds": {"pss_kib": 102400, "single_core_cpu_percent_below": 0.1, "cpu_window_seconds_at_least": 60},
              "trials": results, "all_empty_idle_thresholds_met": all(r["empty_idle_thresholds_met"] for r in results),
              "limitations": ["Empty workspace only; no source/review/history workload or physical Wayland driver",
                              "Sampled process-tree PSS/CPU; short-lived processes and between-sample peaks may be missed",
                              "Repository/pkg-config metadata is observational; binary identity is its independent SHA-256, not loaded-library certification",
                              "Private bus/display infrastructure excluded from application process totals",
                              "This is resource evidence, not full resource certification"]}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    for index, run in enumerate(results, 1):
        print(f"Trial {index}: max sampled process-tree PSS {run['max_observed_pss_kib']/1024:.1f} MiB; "
              f"idle CPU {run['observed_single_core_cpu_percent']:.2f}%")
    print(f"Wrote {args.output}; full resource acceptance remains unproven")
    if args.require_thresholds and not result["all_empty_idle_thresholds_met"]:
        raise SystemExit("Empty-idle observation thresholds/window did not pass")


if __name__ == "__main__":
    main()
