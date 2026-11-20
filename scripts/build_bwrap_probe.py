#!/usr/bin/env python3
"""Build an exact upstream constructor for CI; never install or execute it here."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import urllib.request

URL = 'https://codeload.github.com/containers/bubblewrap/tar.gz/refs/tags/v0.11.0'
SHA256 = 'cfeeb15fcc47d177d195f06fdf0847e93ee3aa6bf46f6ac0a141fa142759e2c3'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit('Constructor output must be a fresh CI artifact')
    with tempfile.TemporaryDirectory(prefix='hye-bwrap-build-') as temporary:
        root = Path(temporary)
        with urllib.request.urlopen(URL, timeout=30) as response:
            data = response.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024 or hashlib.sha256(data).hexdigest() != SHA256:
            raise SystemExit('Pinned constructor source digest/length mismatch')
        archive = root / 'source.tar.gz'
        archive.write_bytes(data)
        # Extraction only after exact hash verification of the reviewed archive.
        # Its sole symlink is LICENSE -> COPYING, inside the private source root.
        subprocess.run(['/usr/bin/tar', '-xzf', str(archive), '-C', str(root)], check=True, timeout=10)
        source = root / 'bubblewrap-0.11.0'
        build = root / 'build'
        subprocess.run(['meson', 'setup', str(build), str(source), '--buildtype=release',
                        '-Dtests=false', '-Dman=disabled', '-Dbash_completion=disabled',
                        '-Dselinux=disabled'], check=True, timeout=60)
        subprocess.run(['ninja', '-C', str(build)], check=True, timeout=60)
        with args.output.open('xb') as output, (build / 'bwrap').open('rb') as binary:
            shutil.copyfileobj(binary, output)
        args.output.chmod(0o700)
        print('Reviewed constructor source:', SHA256)
        print('Built constructor SHA-256:', hashlib.sha256(args.output.read_bytes()).hexdigest())
    print('Temporary constructor originals/build removed')


if __name__ == '__main__':
    main()
