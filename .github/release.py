#!/usr/bin/env python3
"""Validate and publish the fork's GitHub-only release artifacts."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import xml.etree.ElementTree as ET
import zipfile


TARGETS = (
    ("windows-latest", "styluau-windows-x86_64", "x86_64-pc-windows-msvc"),
    ("ubuntu-22.04", "styluau-linux-x86_64", "x86_64-unknown-linux-gnu"),
    ("ubuntu-22.04", "styluau-linux-x86_64-musl", "x86_64-unknown-linux-musl"),
    ("ubuntu-22.04-arm", "styluau-linux-aarch64", "aarch64-unknown-linux-gnu"),
    ("ubuntu-22.04-arm", "styluau-linux-aarch64-musl", "aarch64-unknown-linux-musl"),
    ("macos-latest", "styluau-macos-x86_64", "x86_64-apple-darwin"),
    ("macos-latest", "styluau-macos-aarch64", "aarch64-apple-darwin"),
)
NUMBER = r"(?:0|[1-9][0-9]*)"
VERSION = re.compile(rf"{NUMBER}\.{NUMBER}\.{NUMBER}-horse\.({NUMBER})\.({NUMBER})")


def metadata(cargo, extension):
    version = cargo["package"]["version"]
    match = VERSION.fullmatch(version)
    if cargo["package"]["name"] != "styluau" or not match:
        raise ValueError("Cargo must declare a StyLuau fork version without a leading 'v'.")
    extension_version = f"2.{match[1]}.{match[2]}"
    if extension["name"] != "styluau" or extension["version"] != extension_version:
        raise ValueError(f"The StyLuau extension version must be {extension_version}.")
    return {"version": version, "extension_version": extension_version}


def read_metadata(root):
    return metadata(
        tomllib.loads((root / "Cargo.toml").read_text()),
        json.loads((root / "stylua-vscode/package.json").read_text()),
    )


def asset_names():
    return sorted([f"{name}.zip" for _, name, _ in TARGETS] + ["styluau.vsix"])


def validate_artifacts(folder, release, publisher):
    names = sorted(path.name for path in folder.iterdir())
    if names != asset_names():
        raise ValueError("Release assets must contain every CLI archive and only the universal VSIX.")
    for name in names:
        path = folder / name
        with zipfile.ZipFile(path) as archive:
            if archive.testzip() is not None:
                raise ValueError(f'Archive "{name}" failed its integrity check.')
            if name != "styluau.vsix":
                binary = "styluau.exe" if "windows" in name else "styluau"
                if archive.namelist() != [binary] or not archive.read(binary):
                    raise ValueError(f'Archive "{name}" must contain only the nonempty "{binary}" executable.')
                continue
            manifest = json.loads(archive.read("extension/package.json"))
            if (manifest.get("publisher"), manifest.get("name"), manifest.get("version")) != (
                publisher, "styluau", release["extension_version"]
            ):
                raise ValueError("The VSIX has an unexpected publisher, name, or version.")
            identity = ET.fromstring(archive.read("extension.vsixmanifest")).find(".//{*}Identity")
            if identity is None or any(identity.get(key) != value for key, value in {
                "Publisher": publisher, "Id": "styluau", "Version": release["extension_version"]
            }.items()):
                raise ValueError("The VSIX XML identity does not match its package manifest.")
            for required in ("extension/dist/extension.js", "extension/LICENSE.md"):
                if not archive.read(required):
                    raise ValueError(f'The VSIX must include a nonempty "{required}".')
    return [folder / name for name in names]


def release_notes(changelog, version):
    match = re.search(rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## |\Z)", changelog, re.M | re.S)
    if not match or not match[1].strip():
        raise ValueError("The release must have a nonempty changelog entry.")
    return match[1].strip() + "\n"


def gh(*args):
    result = subprocess.run(["gh", *args], check=True, text=True, capture_output=True)
    return result.stdout


def existing_release(repository, version):
    pages = json.loads(gh("api", f"repos/{repository}/releases", "--paginate", "--slurp"))
    return next((release for page in pages for release in page if release["tag_name"] == version), None)


def verify_tag(repository, version, commit):
    refs = json.loads(gh("api", f"repos/{repository}/git/matching-refs/tags/{version}"))
    ref = next((ref for ref in refs if ref["ref"] == f"refs/tags/{version}"), None)
    if ref is None:
        return
    obj = ref["object"]
    while obj["type"] == "tag":
        obj = json.loads(gh("api", f"repos/{repository}/git/tags/{obj['sha']}"))["object"]
    if obj["type"] != "commit" or obj["sha"] != commit:
        raise ValueError("The release tag already points to a different commit.")


def verify_remote(repository, version, folder):
    with tempfile.TemporaryDirectory(prefix="styluau-release-check-") as temporary:
        gh("release", "download", version, "--repo", repository, "--dir", temporary)
        remote = Path(temporary)
        if sorted(path.name for path in remote.iterdir()) != asset_names():
            raise ValueError("The remote release does not contain the complete asset set.")
        for name in asset_names():
            if (remote / name).read_bytes() != (folder / name).read_bytes():
                raise ValueError(f'Remote asset "{name}" differs from the validated build.')


def publish(repository, commit, release, folder, notes, publisher):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("A full checked-out commit SHA is required.")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("A GitHub owner/repository identifier is required.")
    assets = validate_artifacts(folder, release, publisher)
    version = release["version"]
    verify_tag(repository, version, commit)
    current = existing_release(repository, version)
    if current is not None:
        if current["target_commitish"] != commit:
            raise ValueError("The existing release belongs to a different source commit.")
        if not current["draft"]:
            verify_remote(repository, version, folder)
            return
    else:
        gh("release", "create", version, "--repo", repository, "--target", commit,
           "--title", version, "--notes-file", str(notes), "--draft")
    gh("release", "upload", version, "--repo", repository, "--clobber", *map(str, assets))
    verify_remote(repository, version, folder)
    verify_tag(repository, version, commit)
    gh("release", "edit", version, "--repo", repository, "--draft=false", "--prerelease=false", "--latest")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("metadata", "validate", "publish"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--publisher")
    parser.add_argument("--repository")
    parser.add_argument("--commit")
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()
    release = read_metadata(args.root)
    if args.command == "metadata":
        matrix = {"include": [dict(zip(("os", "artifact", "target"), target)) for target in TARGETS]}
        print(json.dumps({**release, "matrix": matrix}))
        if args.github_output is not None:
            with args.github_output.open("a") as output:
                for key, value in {**release, "matrix": json.dumps(matrix)}.items():
                    output.write(f"{key}={value}\n")
        return
    if args.artifacts is None or not args.publisher:
        parser.error("Artifact validation requires --artifacts and --publisher.")
    validate_artifacts(args.artifacts, release, args.publisher)
    if args.command == "publish":
        if not args.repository or not args.commit:
            parser.error("Publication requires --repository and --commit.")
        notes = release_notes((args.root / "CHANGELOG.md").read_text(), release["version"])
        with tempfile.TemporaryDirectory(prefix="styluau-release-notes-") as temporary:
            note_path = Path(temporary) / "notes.txt"
            note_path.write_text(notes)
            publish(args.repository, args.commit, release, args.artifacts, note_path, args.publisher)
        print(f'GitHub release "{release["version"]}" is published and verified.')
    else:
        print("The complete release artifact set is valid.")


if __name__ == "__main__":
    main()
