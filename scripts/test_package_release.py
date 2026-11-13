"""Check archive layout, launch metadata and the Linux user installer."""

import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tarfile
import tempfile
import unittest

from package_release import TARGETS, package


class PackageReleaseTests(unittest.TestCase):
    def test_platform_packages_preserve_binary_and_include_launch_icons(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            binary.write_bytes(b"test executable")
            for target in TARGETS:
                with self.subTest(target=target):
                    artifact = package(binary, target, work / target)
                    if "windows" in target:
                        self.assertEqual(artifact.read_bytes(), binary.read_bytes())
                        self.assertEqual(artifact.suffix, ".exe")
                        continue
                    with tarfile.open(artifact) as tar:
                        if "apple" in target:
                            root = "Open Process.app/Contents"
                            plist = plistlib.loads(tar.extractfile(f"{root}/Info.plist").read())
                            executable = f"{root}/MacOS/{plist['CFBundleExecutable']}"
                            icon = f"{root}/Resources/{plist['CFBundleIconFile']}"
                            self.assertEqual(plist["CFBundlePackageType"], "APPL")
                            self.assertTrue(tar.extractfile(icon).read().startswith(b"icns"))
                        else:
                            executable = "open-process/open-process"
                            self.assertTrue(tar.extractfile("open-process/open-process.png").read().startswith(b"\x89PNG"))
                            self.assertIn("Icon=open-process", tar.extractfile("open-process/open-process.desktop").read().decode())
                            self.assertTrue(tar.getmember("open-process/install.sh").mode & 0o111)
                        self.assertEqual(tar.extractfile(executable).read(), binary.read_bytes())
                        self.assertTrue(tar.getmember(executable).mode & 0o111)

    @unittest.skipUnless(sys.platform == "linux", "Linux shell installer")
    def test_linux_installer_registers_executable_and_icon_in_custom_directories(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            binary.write_bytes(b"test executable")
            artifact = package(binary, "x86_64-unknown-linux-gnu", work / "out")
            with tarfile.open(artifact) as tar:
                tar.extractall(work / "extracted", filter="data")
            prefix = work / "user applications"
            data = work / "user data"
            subprocess.run(["sh", str(work / "extracted/open-process/install.sh")], check=True, env={
                **os.environ, "OP_INSTALL_PREFIX": str(prefix), "XDG_DATA_HOME": str(data),
            }, capture_output=True)
            executable = prefix / "bin/open-process"
            self.assertEqual(executable.read_bytes(), binary.read_bytes())
            self.assertTrue(os.access(executable, os.X_OK))
            desktop = (data / "applications/open-process.desktop").read_text()
            self.assertIn(f'Exec="{executable}" %F', desktop)
            self.assertTrue((data / "icons/hicolor/512x512/apps/open-process.png").is_file())


if __name__ == "__main__":
    unittest.main()
