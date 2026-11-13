#!/usr/bin/env python3
"""Package native releases with their platform icons; Python standard library only."""

import argparse
from pathlib import Path
import plistlib
import shutil
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "crates/op-app/assets"
TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc")


def package(binary: Path, target: str, output: Path) -> Path:
    if target not in TARGETS:
        raise ValueError(f"Unsupported release target: {target}")
    output.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if "windows" in target else ""
    raw = output / f"open-process-{target}{suffix}"
    shutil.copyfile(binary, raw)
    raw.chmod(0o755)
    if suffix:
        return raw

    archive = output / f"open-process-{target}.tar.gz"
    with tempfile.TemporaryDirectory() as temp:
        staging = Path(temp)
        if "apple" in target:
            app = staging / "Open Process.app"
            contents = app / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            (contents / "Resources").mkdir()
            executable = contents / "MacOS/open-process"
            shutil.copyfile(binary, executable)
            executable.chmod(0o755)
            shutil.copyfile(ASSETS / "icons/open-process.icns", contents / "Resources/open-process.icns")
            with (ROOT / "Cargo.toml").open("rb") as source:
                version = tomllib.load(source)["workspace"]["package"]["version"]
            with (contents / "Info.plist").open("wb") as plist:
                plistlib.dump({
                    "CFBundleDevelopmentRegion": "en",
                    "CFBundleExecutable": "open-process",
                    "CFBundleIconFile": "open-process.icns",
                    "CFBundleIdentifier": "dev.linv.open-process",
                    "CFBundleInfoDictionaryVersion": "6.0",
                    "CFBundleName": "Open Process",
                    "CFBundleDisplayName": "Open Process",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": version,
                    "NSHighResolutionCapable": True,
                    "LSApplicationCategoryType": "public.app-category.productivity",
                }, plist)
            (contents / "PkgInfo").write_bytes(b"APPL????")
            root = app
        else:
            root = staging / "open-process"
            root.mkdir()
            executable = root / "open-process"
            shutil.copyfile(binary, executable)
            executable.chmod(0o755)
            shutil.copyfile(ASSETS / "icons/open-process.png", root / "open-process.png")
            shutil.copyfile(ASSETS / "icons/open-process.svg", root / "open-process.svg")
            for name in ("open-process.desktop", "install.sh"):
                shutil.copyfile(ASSETS / "linux" / name, root / name)
            (root / "install.sh").chmod(0o755)
        for name in ("LICENSE-MIT", "LICENSE-APACHE"):
            destination = root / "Contents/Resources" if "apple" in target else root
            shutil.copyfile(ROOT / name, destination / name)
        with tarfile.open(archive, "w:gz") as tar:
            executables = {executable.relative_to(staging).as_posix(), f"{root.name}/install.sh"}

            def archive_mode(info):
                # Preserve Unix launch permissions even when checking packages on Windows.
                info.mode = 0o755 if info.isdir() or info.name in executables else 0o644
                return info

            tar.add(root, arcname=root.name, filter=archive_mode)
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--output", default=Path("out"), type=Path)
    args = parser.parse_args()
    print(package(args.binary, args.target, args.output))


if __name__ == "__main__":
    main()
