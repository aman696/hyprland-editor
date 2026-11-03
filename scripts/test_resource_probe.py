"""Integration checks for /proc observation and owned process-group cleanup."""
import os
from pathlib import Path
import select
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
import resource_probe as probe


class ResourceObservation(unittest.TestCase):
    def assert_exited(self, pid):
        deadline = time.monotonic() + 3
        while True:
            try:
                state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
            except FileNotFoundError:
                return
            if state == "Z":
                return  # exited child awaits the namespace's reaper; it is not running
            if time.monotonic() >= deadline:
                self.fail("Owned helper remained running after process-group teardown")
            time.sleep(0.02)

    def test_observation_includes_helpers_spawned_by_other_threads(self):
        code = """
import subprocess,sys,threading,time
def start():
    child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(20)'])
    print(child.pid,flush=True)
    time.sleep(20)
thread=threading.Thread(target=start);thread.start()
time.sleep(20)
"""
        process = subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE,
                                   start_new_session=True, text=True)
        try:
            self.assertTrue(select.select([process.stdout], [], [], 5)[0])
            child = int(process.stdout.readline().strip())
            rows = probe.sample(process.pid)
            self.assertIn(child, {row["pid"] for row in rows})
            self.assertTrue(all(row["kib"]["Pss"] > 0 for row in rows))
            identity, ticks = probe.identity(process.pid)
            self.assertEqual(identity[0], process.pid)
            self.assertGreater(identity[1], 0)
            self.assertGreaterEqual(ticks, 0)
        finally:
            probe.stop(process)
            process.stdout.close()
        self.assertIsNotNone(process.poll())
        self.assert_exited(child)

    def test_stop_cleans_the_owned_group_when_its_original_leader_already_exited(self):
        code = "import subprocess,sys; p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(20)']); print(p.pid,flush=True)"
        process = subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE,
                                   start_new_session=True, text=True)
        try:
            self.assertTrue(select.select([process.stdout], [], [], 5)[0])
            child = int(process.stdout.readline().strip())
            process.wait(timeout=5)
            self.assertGreater(probe.identity(child)[0][1], 0)
            probe.stop(process)
            self.assert_exited(child)
        finally:
            probe.stop(process)
            process.stdout.close()

    def test_failed_gui_start_cleans_the_private_fixture(self):
        xvfb = shutil.which("Xvfb") or ".dev/xvfb/usr/bin/Xvfb"
        bus = shutil.which("dbus-run-session")
        if not Path(xvfb).exists() or bus is None:
            self.skipTest("Private display/bus tools are unavailable")
        with tempfile.TemporaryDirectory(prefix="hye-probe-failure-") as directory:
            root = Path(directory)
            env = dict(os.environ, TMPDIR=str(root))
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("resource_probe.py")),
                 "--binary", "/usr/bin/false", "--xvfb", xvfb, "--bus", bus,
                 "--repeat", "1", "--settle-seconds", "0", "--sample-seconds", "1",
                 "--output", str(root / "result.json")],
                env=env, capture_output=True, text=True, timeout=20)
            self.assertNotEqual(result.returncode, 0)
            self.assertRegex(result.stderr, r"GUI (failed|exited|unavailable)")
            self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
