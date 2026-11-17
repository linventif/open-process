"""Check native packages, macOS bundle signatures and the Linux user installer."""

import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from package_release import TARGETS, package


class PackageReleaseTests(unittest.TestCase):
    def test_platform_packages_preserve_binary_and_include_launch_icons(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            binary.write_bytes(b"test executable")
            for target in TARGETS:
                with self.subTest(target=target):
                    if "apple" in target:
                        # Dummy bytes are not Mach-O code. Real signing is tested on macOS below.
                        with patch("package_release.subprocess.run"):
                            artifact = package(binary, target, work / target)
                    else:
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

    def test_macos_signs_complete_bundle_before_archiving(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            binary.write_bytes(b"test executable")

            def codesign(command, *, check):
                self.assertTrue(check)
                app = Path(command[-1])
                contents = app / "Contents"
                plist = plistlib.loads((contents / "Info.plist").read_bytes())
                self.assertEqual((contents / "MacOS" / plist["CFBundleExecutable"]).read_bytes(), binary.read_bytes())
                for name in (plist["CFBundleIconFile"], "LICENSE-MIT", "LICENSE-APACHE"):
                    self.assertTrue((contents / "Resources" / name).is_file())
                seal = contents / "_CodeSignature/CodeResources"
                if command[1] == "--force":
                    self.assertEqual(command[:-1], ["codesign", "--force", "--sign", "-"])
                    seal.parent.mkdir()
                    seal.write_bytes(b"test resource seal")
                else:
                    self.assertEqual(command[:-1], ["codesign", "--verify", "--deep", "--strict"])
                    self.assertTrue(seal.is_file())

            for target in (target for target in TARGETS if "apple" in target):
                with self.subTest(target=target), patch("package_release.subprocess.run", side_effect=codesign) as run:
                    artifact = package(binary, target, work / target)
                    self.assertEqual(run.call_count, 2)
                    with tarfile.open(artifact) as tar:
                        self.assertEqual(
                            tar.extractfile("Open Process.app/Contents/_CodeSignature/CodeResources").read(),
                            b"test resource seal",
                        )

    def test_macos_does_not_create_archive_when_signing_or_verification_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            binary.write_bytes(b"test executable")
            for step in ("sign", "verify"):
                with self.subTest(step=step):
                    failure = subprocess.CalledProcessError(1, "codesign")
                    effects = [failure] if step == "sign" else [None, failure]
                    output = work / step
                    with patch("package_release.subprocess.run", side_effect=effects):
                        with self.assertRaises(subprocess.CalledProcessError):
                            package(binary, "aarch64-apple-darwin", output)
                    self.assertFalse(list(output.glob("*.tar.gz")))

    @unittest.skipUnless(sys.platform == "darwin", "Requires Apple's codesign and C compiler")
    def test_macos_signature_survives_extraction_and_detects_resource_tampering(self):
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp)
            binary = work / "binary"
            subprocess.run(
                ["xcrun", "clang", "-x", "c", "-o", str(binary), "-"],
                input=b"int main(void) { return 0; }\n", check=True, capture_output=True,
            )
            target = "aarch64-apple-darwin" if os.uname().machine == "arm64" else "x86_64-apple-darwin"
            artifact = package(binary, target, work / "out")
            extracted = work / "extracted"
            with tarfile.open(artifact) as tar:
                tar.extractall(extracted, filter="data")
            app = extracted / "Open Process.app"
            verify = ["codesign", "--verify", "--deep", "--strict", str(app)]
            subprocess.run(verify, check=True, capture_output=True)
            subprocess.run([str(app / "Contents/MacOS/open-process")], check=True, capture_output=True)
            resource = app / "Contents/Resources/LICENSE-MIT"
            resource.write_bytes(resource.read_bytes() + b"\nmodified\n")
            self.assertNotEqual(subprocess.run(verify, capture_output=True).returncode, 0)

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
