"""Metadata regression checks only; ordinary directories cannot prove cgroup authority."""
from pathlib import Path
import tempfile
import unittest
import cgroup_kernel_probe as probe


class PrerequisiteChecks(unittest.TestCase):
    def fixture(self, root):
        path = Path(root)
        for name in ['cgroup.controllers', 'cgroup.subtree_control']:
            (path / name).write_text('cpu memory pids\n')
        return path

    def test_global_root_without_type_file_is_read_only_valid_metadata(self):
        with tempfile.TemporaryDirectory(prefix='hye-root-shape-') as directory:
            path = self.fixture(directory)
            before = {p.name: p.read_bytes() for p in path.iterdir()}
            probe.metadata_prerequisites(path)
            self.assertEqual(before, {p.name: p.read_bytes() for p in path.iterdir()})

    def test_present_normal_domain_is_accepted_but_other_types_are_refused(self):
        with tempfile.TemporaryDirectory(prefix='hye-domain-shape-') as directory:
            path = self.fixture(directory)
            (path / 'cgroup.type').write_text('domain\n')
            probe.metadata_prerequisites(path)
            for kind in ['threaded', 'domain threaded', 'domain invalid', '']:
                (path / 'cgroup.type').write_text(kind)
                before = {p.name: p.read_bytes() for p in path.iterdir()}
                with self.assertRaises(RuntimeError):
                    probe.metadata_prerequisites(path)
                self.assertEqual(before, {p.name: p.read_bytes() for p in path.iterdir()})

    def test_missing_available_or_enabled_controller_never_writes_an_enablement(self):
        with tempfile.TemporaryDirectory(prefix='hye-controller-shape-') as directory:
            path = self.fixture(directory)
            for control in ['cgroup.controllers', 'cgroup.subtree_control']:
                for missing in probe.REQUIRED:
                    self.fixture(directory)
                    (path / control).write_text(' '.join(sorted(probe.REQUIRED - {missing})))
                    before = {p.name: p.read_bytes() for p in path.iterdir()}
                    with self.assertRaises(RuntimeError):
                        probe.metadata_prerequisites(path)
                    self.assertEqual(before, {p.name: p.read_bytes() for p in path.iterdir()})

    def test_type_read_errors_are_not_treated_as_a_global_root_absence(self):
        with tempfile.TemporaryDirectory(prefix='hye-type-error-') as directory:
            path = self.fixture(directory)
            (path / 'cgroup.type').mkdir()
            with self.assertRaises(IsADirectoryError):
                probe.metadata_prerequisites(path)
            self.assertEqual((path / 'cgroup.subtree_control').read_text(), 'cpu memory pids\n')


if __name__ == '__main__':
    unittest.main()
