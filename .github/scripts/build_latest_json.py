#!/usr/bin/env python3
"""Build one multi-platform Tauri updater manifest from a release directory.

Tauri's GitHub Action writes ``latest.json`` from whichever matrix job
finishes last, so parallel Windows and macOS builds overwrite each other.
This script rebuilds the manifest from the finished release assets:

* ``windows-x86_64`` and ``windows-<arch>-nsis`` point at the NSIS installer
  (the ``.exe`` when ``createUpdaterArtifacts`` is true, otherwise the
  ``.nsis.zip``).
* ``darwin-aarch64`` / ``darwin-x86_64`` and the ``-app`` keys the updater
  checks first point at the ``.app.tar.gz`` updater archives.

The GitHub copy keeps ``releases/download`` URLs. The Qiniu copy rewrites
every asset URL to ``https://dl.yzre.cn/toolbox/<filename>``.
"""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from datetime import datetime
from pathlib import Path

REQUIRED_PLATFORMS = (
    "darwin-aarch64",
    "darwin-aarch64-app",
    "darwin-x86_64",
    "darwin-x86_64-app",
    "windows-x86_64",
    "windows-x86_64-nsis",
)

ARCH_ALIASES = {
    "aarch64": "aarch64",
    "arm64": "aarch64",
    "x64": "x86_64",
    "x86_64": "x86_64",
    "amd64": "x86_64",
    "x86": "i686",
    "i686": "i686",
    "universal": "universal",
}


class ManifestError(Exception):
    """The release assets cannot form a complete updater manifest."""


def _read_signature(path: Path) -> str:
    if not path.is_file():
        raise ManifestError(f"missing signature {path.name}")
    signature = path.read_text(encoding="utf-8").rstrip("\r\n")
    if len(signature) < 32 or any(char.isspace() for char in signature):
        raise ManifestError(f"signature {path.name} is empty or not a single line")
    return signature


def _arch_token(stem: str) -> str:
    token = stem.rsplit("_", 1)[-1]
    if token.endswith("-setup"):
        token = token[: -len("-setup")]
    arch = ARCH_ALIASES.get(token)
    if arch is None:
        raise ManifestError(f"cannot tell the CPU architecture of {stem}")
    return arch


def _entry(base: str, filename: str, signature: str) -> dict[str, str]:
    if "/" in filename or filename in {"", ".", ".."}:
        raise ManifestError(f"refusing asset name {filename!r}")
    return {"signature": signature, "url": f"{base.rstrip('/')}/{filename}"}


def _prefer_windows(artifacts: dict[str, tuple[str, str]]) -> tuple[str, str]:
    """v2 updater installs the NSIS exe directly; the zip is the v1 bundle."""
    if "exe" in artifacts:
        return artifacts["exe"]
    return artifacts["zip"]


def build_platforms(assets_dir: Path, download_base: str) -> dict[str, dict[str, str]]:
    windows: dict[str, dict[str, tuple[str, str]]] = {}
    macos: dict[str, tuple[str, str]] = {}
    universal: tuple[str, str] | None = None

    for path in sorted(assets_dir.iterdir()):
        if not path.is_file():
            continue
        name = path.name
        if name == "latest.json" or name.endswith(".sig") or name.endswith(".dmg"):
            continue

        if name.endswith(".app.tar.gz"):
            arch = _arch_token(name[: -len(".app.tar.gz")])
            signed = (name, _read_signature(path.with_name(name + ".sig")))
            if arch == "universal":
                universal = signed
            else:
                if arch in macos:
                    raise ManifestError(f"more than one macOS updater archive for {arch}")
                macos[arch] = signed
            continue

        if name.endswith("-setup.exe") or name.endswith(".nsis.zip"):
            kind = "exe" if name.endswith("-setup.exe") else "zip"
            arch = _arch_token(name[: -len(".exe")] if kind == "exe" else name[: -len(".nsis.zip")])
            signed = (name, _read_signature(path.with_name(name + ".sig")))
            slot = windows.setdefault(arch, {})
            if kind in slot:
                raise ManifestError(f"more than one Windows {kind} installer for {arch}")
            slot[kind] = signed

    if universal is not None:
        for arch in ("aarch64", "x86_64"):
            macos.setdefault(arch, universal)

    platforms: dict[str, dict[str, str]] = {}
    for arch, signed in macos.items():
        if arch == "universal":
            continue
        entry = _entry(download_base, signed[0], signed[1])
        platforms[f"darwin-{arch}"] = dict(entry)
        platforms[f"darwin-{arch}-app"] = dict(entry)

    for arch, kinds in windows.items():
        filename, signature = _prefer_windows(kinds)
        entry = _entry(download_base, filename, signature)
        platforms[f"windows-{arch}"] = dict(entry)
        platforms[f"windows-{arch}-nsis"] = dict(entry)

    missing = [key for key in REQUIRED_PLATFORMS if key not in platforms]
    if missing:
        raise ManifestError(
            "updater manifest is missing "
            + ", ".join(missing)
            + "; expected Windows NSIS installers and macOS .app.tar.gz archives"
        )
    return platforms


def render_manifest(
    assets_dir: Path,
    version: str,
    notes: str,
    pub_date: str,
    download_base: str,
) -> dict:
    version = version[1:] if version.startswith("v") else version
    if not version or version.startswith("."):
        raise ManifestError(f"invalid updater version {version!r}")
    try:
        datetime.fromisoformat(pub_date.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ManifestError(f"pub_date is not RFC3339: {pub_date}") from exc

    platforms = build_platforms(assets_dir, download_base)
    ordered = {key: platforms[key] for key in sorted(platforms)}
    return {
        "version": version,
        "notes": notes,
        "pub_date": pub_date,
        "platforms": ordered,
    }


def write_manifest(path: Path, manifest: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def check_manifest(path: Path, url_prefix: str) -> None:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    platforms = manifest.get("platforms")
    if not isinstance(platforms, dict):
        raise ManifestError(f"{path} has no platforms object")
    missing = [key for key in REQUIRED_PLATFORMS if key not in platforms]
    if missing:
        raise ManifestError(f"{path} is missing {', '.join(missing)}")
    prefix = url_prefix.rstrip("/") + "/"
    for key, entry in platforms.items():
        url = entry.get("url", "")
        signature = entry.get("signature", "")
        if not url.startswith(prefix):
            raise ManifestError(f"{key} url does not start with {prefix}: {url}")
        if "github.com" in url and "dl.yzre.cn" in prefix:
            raise ManifestError(f"{key} still points at GitHub: {url}")
        if not signature or not isinstance(signature, str):
            raise ManifestError(f"{key} has no signature")
    print(f"checked {path.name}: {', '.join(sorted(platforms))}")


def _write_asset(directory: Path, name: str, signature: str) -> None:
    (directory / name).write_bytes(b"artifact")
    (directory / f"{name}.sig").write_text(signature + "\n", encoding="utf-8")


def self_test() -> None:
    signature = "d" * 48
    with tempfile.TemporaryDirectory() as tmp:
        assets = Path(tmp)
        _write_asset(assets, "toolbox_0.5.0_x64-setup.exe", signature + "win")
        _write_asset(assets, "toolbox_0.5.0_x64-setup.nsis.zip", signature + "zip")
        _write_asset(assets, "toolbox_0.5.0_aarch64.app.tar.gz", signature + "arm")
        _write_asset(assets, "toolbox_0.5.0_x64.app.tar.gz", signature + "x64")
        _write_asset(assets, "toolbox_0.5.0_aarch64.dmg", signature + "dmg")
        _write_asset(assets, "toolbox_0.5.0_x64.dmg", signature + "dmg")

        github = render_manifest(
            assets,
            "v0.5.0",
            "See the assets to download and install.",
            "2026-09-09T07:23:50Z",
            "https://github.com/yunfengnobug/toolbox/releases/download/v0.5.0",
        )
        cdn = render_manifest(
            assets,
            "0.5.0",
            github["notes"],
            github["pub_date"],
            "https://dl.yzre.cn/toolbox",
        )
        assert github["version"] == "0.5.0"
        assert set(github["platforms"]) == set(REQUIRED_PLATFORMS)
        windows = github["platforms"]["windows-x86_64"]
        assert windows["url"].endswith("/toolbox_0.5.0_x64-setup.exe")
        assert github["platforms"]["windows-x86_64-nsis"] == windows
        assert windows["signature"] == signature + "win"
        assert github["platforms"]["darwin-aarch64"]["url"].endswith(
            "/toolbox_0.5.0_aarch64.app.tar.gz"
        )
        assert github["platforms"]["darwin-aarch64-app"] == github["platforms"]["darwin-aarch64"]
        assert github["platforms"]["darwin-x86_64"]["url"].endswith(
            "/toolbox_0.5.0_x64.app.tar.gz"
        )
        assert cdn["platforms"]["darwin-x86_64-app"]["url"] == (
            "https://dl.yzre.cn/toolbox/toolbox_0.5.0_x64.app.tar.gz"
        )
        assert cdn["platforms"]["darwin-x86_64-app"]["signature"] == (
            github["platforms"]["darwin-x86_64"]["signature"]
        )
        assert "github.com" not in json.dumps(cdn["platforms"])

        github_path = assets / "latest.json"
        cdn_path = assets / "cdn-latest.json"
        write_manifest(github_path, github)
        write_manifest(cdn_path, cdn)
        check_manifest(
            github_path,
            "https://github.com/yunfengnobug/toolbox/releases/download/v0.5.0/",
        )
        check_manifest(cdn_path, "https://dl.yzre.cn/toolbox/")
        try:
            check_manifest(github_path, "https://dl.yzre.cn/toolbox/")
        except ManifestError:
            pass
        else:
            raise AssertionError("github manifest was accepted as a CDN manifest")

        zip_only = Path(tmp) / "zip-only"
        zip_only.mkdir()
        _write_asset(zip_only, "toolbox_0.5.0_x64-setup.nsis.zip", signature + "zip")
        _write_asset(zip_only, "toolbox_0.5.0_aarch64.app.tar.gz", signature + "arm")
        _write_asset(zip_only, "toolbox_0.5.0_x64.app.tar.gz", signature + "x64")
        zipped = render_manifest(zip_only, "0.5.0", "", "2026-09-09T07:23:50Z", "https://dl.yzre.cn/toolbox")
        assert zipped["platforms"]["windows-x86_64-nsis"]["url"].endswith(".nsis.zip")

        incomplete = Path(tmp) / "incomplete"
        incomplete.mkdir()
        _write_asset(incomplete, "toolbox_0.5.0_x64-setup.exe", signature + "win")
        try:
            render_manifest(incomplete, "0.5.0", "", "2026-09-09T07:23:50Z", "https://dl.yzre.cn/toolbox")
        except ManifestError as exc:
            if "darwin-aarch64" not in str(exc):
                raise
        else:
            raise AssertionError("incomplete manifest was accepted")

    print("self-test ok")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--assets-dir", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--notes-file", type=Path)
    parser.add_argument("--pub-date")
    parser.add_argument("--github-base")
    parser.add_argument("--cdn-base", default="https://dl.yzre.cn/toolbox")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--cdn-output", type=Path)
    parser.add_argument("--check", type=Path)
    parser.add_argument("--url-prefix")
    args = parser.parse_args(argv)

    try:
        if args.self_test:
            self_test()
            return 0
        if args.check:
            if not args.url_prefix:
                raise ManifestError("--check requires --url-prefix")
            check_manifest(args.check, args.url_prefix)
            return 0
        if not all([args.assets_dir, args.version, args.pub_date, args.github_base, args.github_output, args.cdn_output]):
            raise ManifestError("missing arguments; pass --self-test, --check, or the full build options")
        notes = ""
        if args.notes_file:
            notes = args.notes_file.read_text(encoding="utf-8")
            if notes.endswith("\n"):
                notes = notes[:-1]
        github = render_manifest(args.assets_dir, args.version, notes, args.pub_date, args.github_base)
        cdn = render_manifest(args.assets_dir, args.version, notes, args.pub_date, args.cdn_base)
        write_manifest(args.github_output, github)
        write_manifest(args.cdn_output, cdn)
        for key in REQUIRED_PLATFORMS:
            filename = github["platforms"][key]["url"].rsplit("/", 1)[-1]
            print(f"{key} -> {filename}")
        return 0
    except ManifestError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
