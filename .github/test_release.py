import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import release


VERSION = "2.6.0-horse.1.0"
COMMIT = "a" * 40
PUBLISHER = "example"
METADATA = {"version": VERSION, "extension_version": "2.1.0"}


def write_assets(folder, publisher=PUBLISHER, version="2.1.0"):
    for name in release.asset_names():
        with zipfile.ZipFile(folder / name, "w") as archive:
            if name == "styluau.vsix":
                archive.writestr("extension/package.json", json.dumps({
                    "publisher": publisher, "name": "styluau", "version": version
                }))
                archive.writestr("extension.vsixmanifest", (
                    '<PackageManifest xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">'
                    f'<Metadata><Identity Id="styluau" Publisher="{publisher}" Version="{version}" />'
                    '</Metadata></PackageManifest>'
                ))
                archive.writestr("extension/dist/extension.js", "exports.activate = () => {};")
                archive.writestr("extension/LICENSE.md", "License text.")
            else:
                archive.writestr("styluau.exe" if "windows" in name else "styluau", b"executable")


class GitHub:
    def __init__(self):
        self.current = None
        self.assets = {}
        self.fail_upload = False
        self.tag = []
        self.changes = []

    def __call__(self, *args):
        if args[0] == "api":
            if "matching-refs" in args[1]:
                return json.dumps(self.tag)
            return json.dumps([[self.current]] if self.current else [[]])
        action = args[1]
        if action == "download":
            destination = Path(args[args.index("--dir") + 1])
            for name, data in self.assets.items():
                (destination / name).write_bytes(data)
            return ""
        self.changes.append(action)
        if action == "create":
            self.current = {"tag_name": args[2], "draft": True, "target_commitish": COMMIT}
        elif action == "upload":
            if self.fail_upload:
                raise RuntimeError("The upload failed.")
            paths = args[args.index("--clobber") + 1:]
            self.assets.update({Path(path).name: Path(path).read_bytes() for path in paths})
        elif action == "edit":
            self.current["draft"] = False
        else:
            raise AssertionError(args)
        return ""


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.folder = Path(self.temporary.name)
        self.assets = self.folder / "assets"
        self.assets.mkdir()
        write_assets(self.assets)
        self.notes = self.folder / "notes.txt"
        self.notes.write_text("Release notes.\n")

    def publish(self, github):
        with patch.object(release, "gh", github):
            release.publish("example/formatter", COMMIT, METADATA, self.assets, self.notes, PUBLISHER)

    def test_metadata_is_pure_and_maps_fork_version(self):
        cargo = {"package": {"name": "styluau", "version": VERSION}}
        extension = {"name": "styluau", "version": "2.1.0"}
        before = copy.deepcopy((cargo, extension))
        self.assertEqual(release.metadata(cargo, extension), METADATA)
        self.assertEqual(release.metadata(cargo, extension), METADATA)
        self.assertEqual((cargo, extension), before)
        cargo["package"]["version"] = "2.6.0-horse.12.34"
        extension["version"] = "2.12.34"
        self.assertEqual(release.metadata(cargo, extension)["extension_version"], "2.12.34")

    def test_metadata_rejects_unversioned_or_mismatched_packages(self):
        for version in ("v" + VERSION, "2.6.0", "2.6.0-horse.01.0", "2.6.0-horse.1", VERSION + "\n"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                release.metadata({"package": {"name": "styluau", "version": version}},
                                 {"name": "styluau", "version": "2.1.0"})
        for name, version in (("stylua", "2.1.0"), ("styluau", "1.7.2")):
            with self.subTest(name=name, version=version), self.assertRaises(ValueError):
                release.metadata({"package": {"name": "styluau", "version": VERSION}},
                                 {"name": name, "version": version})

    def test_asset_contract_has_all_supported_platforms_and_universal_vsix(self):
        names = [path.name for path in release.validate_artifacts(self.assets, METADATA, PUBLISHER)]
        self.assertEqual(names, sorted({
            "styluau-windows-x86_64.zip", "styluau-linux-x86_64.zip",
            "styluau-linux-x86_64-musl.zip", "styluau-linux-aarch64.zip",
            "styluau-linux-aarch64-musl.zip", "styluau-macos-x86_64.zip",
            "styluau-macos-aarch64.zip", "styluau.vsix",
        }))
        self.assertEqual(len(release.TARGETS), 7)

    def test_missing_or_unexpected_asset_is_rejected(self):
        path = self.assets / "styluau-windows-x86_64.zip"
        original = path.read_bytes()
        path.unlink()
        with self.assertRaises(ValueError):
            release.validate_artifacts(self.assets, METADATA, PUBLISHER)
        path.write_bytes(original)
        (self.assets / "unexpected.zip").write_bytes(b"unexpected")
        with self.assertRaises(ValueError):
            release.validate_artifacts(self.assets, METADATA, PUBLISHER)

    def test_archive_requires_correct_executable_and_no_extra_paths(self):
        path = self.assets / "styluau-linux-x86_64.zip"
        for files in ({"stylua": b"executable"}, {"styluau": b""},
                      {"styluau": b"executable", "../escape": b"unexpected"}):
            with self.subTest(files=files):
                with zipfile.ZipFile(path, "w") as archive:
                    for name, content in files.items():
                        archive.writestr(name, content)
                with self.assertRaises(ValueError):
                    release.validate_artifacts(self.assets, METADATA, PUBLISHER)

    def test_vsix_identity_must_match_publisher_and_version(self):
        for publisher, version in (("upstream", "2.1.0"), (PUBLISHER, "1.7.2")):
            write_assets(self.assets, publisher, version)
            with self.subTest(publisher=publisher, version=version), self.assertRaises(ValueError):
                release.validate_artifacts(self.assets, METADATA, PUBLISHER)

    def test_notes_are_only_the_requested_release(self):
        changelog = f"# Changelog\n\n## [Unreleased]\n\n## [{VERSION}] - 2026-11-19\n\n### Added\n\n- Added a feature.\n\n## [2.6.0]\n\nOld notes.\n"
        self.assertEqual(release.release_notes(changelog, VERSION), "### Added\n\n- Added a feature.\n")
        with self.assertRaises(ValueError):
            release.release_notes(changelog, "2.6.0-horse.9.0")
        with self.assertRaises(ValueError):
            release.release_notes(f"## [{VERSION}]\n\n", VERSION)

    def test_publication_verifies_all_assets_before_publishing(self):
        github = GitHub()
        self.publish(github)
        self.assertFalse(github.current["draft"])
        self.assertEqual(sorted(github.assets), release.asset_names())
        self.assertEqual(github.changes, ["create", "upload", "edit"])

    def test_failed_upload_stays_draft_and_can_be_retried(self):
        github = GitHub()
        github.fail_upload = True
        with self.assertRaises(RuntimeError):
            self.publish(github)
        self.assertTrue(github.current["draft"])
        github.fail_upload = False
        self.publish(github)
        self.assertFalse(github.current["draft"])
        self.assertEqual(github.changes.count("create"), 1)

    def test_retry_never_mutates_a_published_release(self):
        github = GitHub()
        self.publish(github)
        before = copy.deepcopy((github.current, github.assets, github.changes))
        self.publish(github)
        self.assertEqual((github.current, github.assets, github.changes), before)
        github.assets["styluau.vsix"] = b"a different published build"
        before = copy.deepcopy((github.current, github.assets, github.changes))
        with self.assertRaises(ValueError):
            self.publish(github)
        self.assertEqual((github.current, github.assets, github.changes), before)

    def test_incomplete_assets_cannot_create_a_release(self):
        github = GitHub()
        (self.assets / "styluau.vsix").unlink()
        with self.assertRaises(ValueError):
            self.publish(github)
        self.assertIsNone(github.current)
        self.assertEqual(github.changes, [])

    def test_corrupt_remote_upload_is_not_published(self):
        github = GitHub()
        original = github.__call__

        def corrupt_upload(*args):
            result = original(*args)
            if args[:2] == ("release", "upload"):
                github.assets["styluau.vsix"] = b"corrupted during transfer"
            return result

        with patch.object(release, "gh", corrupt_upload), self.assertRaises(ValueError):
            release.publish("example/formatter", COMMIT, METADATA, self.assets, self.notes, PUBLISHER)
        self.assertTrue(github.current["draft"])
        self.assertNotIn("edit", github.changes)

    def test_existing_release_or_tag_from_another_commit_is_untouched(self):
        for draft in (False, True):
            github = GitHub()
            github.current = {"tag_name": VERSION, "draft": draft, "target_commitish": "b" * 40}
            with self.subTest(draft=draft), self.assertRaises(ValueError):
                self.publish(github)
            self.assertEqual(github.changes, [])
        github = GitHub()
        github.tag = [{"ref": f"refs/tags/{VERSION}", "object": {"type": "commit", "sha": "b" * 40}}]
        with self.assertRaises(ValueError):
            self.publish(github)
        self.assertIsNone(github.current)


if __name__ == "__main__":
    unittest.main()
