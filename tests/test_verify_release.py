"""Synthetic package and Git regressions; no release binary is executed."""

import gzip
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "verify_release", Path(__file__).resolve().parents[1] / "scripts/verify_release.py")
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.package = Path(self.temporary.name)
        self.files = {name: (name + "\n").encode() for name in verify.MEMBERS - {"SHA256SUMS"}}
        self.candidate = "a" * 40
        self.binary_sha = verify.sha(self.files["bin/ontodoc"])

    def package_fixture(self, *, extra=(), transform=None, mode=0o755,
                        checksum=None, binary_sha=None):
        files = dict(self.files)
        files["SHA256SUMS"] = b"".join(
            f"{verify.sha(data)}  {name}\n".encode() for name, data in sorted(files.items()))
        if checksum is not None:
            files["SHA256SUMS"] = checksum(files["SHA256SUMS"])
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            for name, data in sorted(files.items()):
                item = tarfile.TarInfo(verify.NAME + "/" + name)
                item.size = len(data)
                item.mode = mode if name == "bin/ontodoc" else 0o644
                if transform:
                    transform(name, item)
                archive.addfile(item, io.BytesIO(data))
            for item, data in extra:
                archive.addfile(item, io.BytesIO(data))
        content = gzip.compress(raw.getvalue(), mtime=0)
        archive_sha = verify.sha(content)
        (self.package / (verify.NAME + ".tar.gz")).write_bytes(content)
        (self.package / "SHA256SUMS").write_text(
            f"{archive_sha}  {verify.NAME}.tar.gz\n")
        receipt = {
            "archive": verify.NAME + ".tar.gz", "archive_sha256": archive_sha,
            "candidate_commit": self.candidate, "binary_sha256": binary_sha or self.binary_sha,
            "files": {name: {"bytes": len(data), "sha256": verify.sha(data)}
                      for name, data in files.items()},
        }
        (self.package / "validation.json").write_text(json.dumps(receipt))
        return {"candidate": self.candidate, "binary_sha": binary_sha or self.binary_sha,
                "archive_sha": archive_sha}

    def test_happy_path_reads_only(self):
        identities = self.package_fixture()
        before = {path.name: path.read_bytes() for path in self.package.iterdir()}
        result = verify.verify_package(self.package, **identities)
        self.assertEqual(result["regular_members"], 9)
        self.assertEqual(before, {path.name: path.read_bytes() for path in self.package.iterdir()})

    def test_corrupt_archive(self):
        identities = self.package_fixture()
        path = self.package / (verify.NAME + ".tar.gz")
        path.write_bytes(path.read_bytes() + b"corrupt")
        with self.assertRaisesRegex(verify.VerificationError, "archive hash"):
            verify.verify_package(self.package, **identities)

    def test_external_checksum(self):
        identities = self.package_fixture()
        (self.package / "SHA256SUMS").write_text(f"{'0' * 64}  {verify.NAME}.tar.gz\n")
        with self.assertRaisesRegex(verify.VerificationError, "external archive checksum"):
            verify.verify_package(self.package, **identities)

    def test_internal_checksum_and_coverage(self):
        for change in (lambda data: b"0" * 64 + data[64:],
                       lambda data: data.split(b"\n", 1)[1],
                       lambda data: data + data.splitlines(keepends=True)[0]):
            with self.subTest(change=change):
                identities = self.package_fixture(checksum=change)
                with self.assertRaises(verify.VerificationError):
                    verify.verify_package(self.package, **identities)

    def test_binary_hash_and_executable(self):
        identities = self.package_fixture(binary_sha="0" * 64)
        with self.assertRaisesRegex(verify.VerificationError, "binary hash"):
            verify.verify_package(self.package, **identities)
        identities = self.package_fixture(mode=0o644)
        with self.assertRaisesRegex(verify.VerificationError, "not executable"):
            verify.verify_package(self.package, **identities)

    def test_receipt_size_hash_coverage_and_identity(self):
        for field in ("bytes", "sha256", "coverage", "candidate_commit"):
            with self.subTest(field=field):
                identities = self.package_fixture()
                path = self.package / "validation.json"
                receipt = json.loads(path.read_text())
                if field == "coverage":
                    del receipt["files"]["NOTICE"]
                elif field == "candidate_commit":
                    receipt[field] = "b" * 40
                else:
                    receipt["files"]["NOTICE"][field] = 999 if field == "bytes" else "0" * 64
                path.write_text(json.dumps(receipt))
                with self.assertRaises(verify.VerificationError):
                    verify.verify_package(self.package, **identities)

    def test_duplicate_receipt_keys(self):
        identities = self.package_fixture()
        (self.package / "validation.json").write_text('{"files":{},"files":{}}')
        with self.assertRaisesRegex(verify.VerificationError, "duplicate receipt"):
            verify.verify_package(self.package, **identities)

    def test_duplicate_traversal_absolute_extra_and_special_members(self):
        cases = ((verify.NAME + "/NOTICE", tarfile.REGTYPE),
                 (verify.NAME + "/../NOTICE", tarfile.REGTYPE),
                 ("/" + verify.NAME + "/NOTICE", tarfile.REGTYPE),
                 (verify.NAME + "/extra", tarfile.REGTYPE),
                 (verify.NAME + "/link", tarfile.SYMTYPE),
                 (verify.NAME + "/link", tarfile.LNKTYPE),
                 (verify.NAME + "/fifo", tarfile.FIFOTYPE),
                 (verify.NAME + "/directory", tarfile.DIRTYPE),
                 (verify.NAME + "/metadata", tarfile.XHDTYPE))
        for name, kind in cases:
            with self.subTest(name=name, kind=kind):
                item = tarfile.TarInfo(name)
                item.type = kind
                item.linkname = "NOTICE" if kind in (tarfile.SYMTYPE, tarfile.LNKTYPE) else ""
                identities = self.package_fixture(extra=[(item, b"")])
                with self.assertRaises(verify.VerificationError):
                    verify.verify_package(self.package, **identities)

    def test_missing_member(self):
        identities = self.package_fixture(transform=lambda name, item: setattr(
            item, "name", verify.NAME + "/README.md") if name == "NOTICE" else None)
        with self.assertRaises(verify.VerificationError):
            verify.verify_package(self.package, **identities)

    def test_limits(self):
        for constant in ("MAX_ARCHIVE", "MAX_MEMBER", "MAX_TOTAL", "MAX_RECEIPT"):
            with self.subTest(constant=constant):
                identities = self.package_fixture()
                with patch.object(verify, constant, 8):
                    with self.assertRaises(verify.VerificationError):
                        verify.verify_package(self.package, **identities)

    def test_truncated_tar_and_trailing_data(self):
        self.package_fixture()
        content = (self.package / (verify.NAME + ".tar.gz")).read_bytes()
        raw = gzip.decompress(content)
        for broken in (raw[:512], raw + b"bad"):
            with self.subTest(length=len(broken)):
                with self.assertRaises(verify.VerificationError):
                    verify.archive_members(gzip.compress(broken))

    def test_package_symlink(self):
        identities = self.package_fixture()
        path = self.package / "validation.json"
        content = path.read_bytes()
        path.unlink()
        (self.package / "other").write_bytes(content)
        path.symlink_to("other")
        with self.assertRaises(OSError):
            verify.verify_package(self.package, **identities)


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.repo = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        (self.repo / "src/nested").mkdir(parents=True)
        for path in (*verify.ROOT_FILES, "src/lib.rs", "src/nested/data.txt"):
            (self.repo / path).write_text(path + "\n")
        self.git("add", ".")
        self.git("commit", "-qm", "candidate")
        self.candidate = self.git("rev-parse", "HEAD").strip()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.repo), *arguments], text=True,
                                       stderr=subprocess.DEVNULL)

    def verify(self, tag=None):
        return verify.verify_sources(self.repo, candidate=self.candidate, tag=tag)

    def test_happy_path_includes_non_rust(self):
        self.assertEqual(self.verify()["regular_files"], 5)

    def test_source_add_delete_change(self):
        for action in ("add", "delete", "change"):
            with self.subTest(action=action):
                path = self.repo / "src/nested/data.txt"
                if action == "add":
                    path = self.repo / "src/untracked.bin"
                    path.write_bytes(b"untracked")
                elif action == "delete":
                    path.unlink()
                else:
                    path.write_bytes(b"changed")
                with self.assertRaises(verify.VerificationError):
                    self.verify()
                if action == "add":
                    path.unlink()
                else:
                    path.write_text("src/nested/data.txt\n")

    def test_root_source_changes(self):
        for name in verify.ROOT_FILES:
            with self.subTest(name=name):
                path = self.repo / name
                original = path.read_bytes()
                path.write_bytes(b"changed")
                with self.assertRaisesRegex(verify.VerificationError, "source bytes"):
                    self.verify()
                path.write_bytes(original)

    def test_source_symlink_directory_symlink_and_special(self):
        path = self.repo / "src/unsafe"
        for kind in ("file", "directory", "fifo"):
            with self.subTest(kind=kind):
                if kind == "fifo":
                    os.mkfifo(path)
                else:
                    path.symlink_to("lib.rs" if kind == "file" else "nested")
                with self.assertRaises(verify.VerificationError):
                    self.verify()
                path.unlink()

    def test_candidate_symlink(self):
        (self.repo / "src/link").symlink_to("lib.rs")
        self.git("add", ".")
        self.git("commit", "-qm", "unsafe candidate")
        candidate = self.git("rev-parse", "HEAD").strip()
        with self.assertRaisesRegex(verify.VerificationError, "candidate contains symlink"):
            verify.verify_sources(self.repo, candidate=candidate)

    def test_annotated_tag_and_out_of_scope_commit(self):
        (self.repo / "notes.md").write_text("not in the source scope\n")
        self.git("add", ".")
        self.git("commit", "-qm", "notes")
        self.git("tag", "-am", "release", "v0.1.0")
        result = self.verify("v0.1.0")
        self.assertEqual(result["annotated_tag"], "v0.1.0")
        self.assertNotEqual(result["tag_commit"], self.candidate)

    def test_lightweight_and_revision_expression_rejected(self):
        self.git("tag", "lightweight")
        for tag in ("lightweight", "HEAD", "lightweight^{commit}", "refs/tags/lightweight", "../bad"):
            with self.subTest(tag=tag):
                with self.assertRaises(verify.VerificationError):
                    self.verify(tag)

    def test_mismatched_annotated_tag(self):
        for change in ("bytes", "inventory"):
            with self.subTest(change=change):
                path = self.repo / ("src/lib.rs" if change == "bytes" else "src/new.txt")
                path.write_text("changed\n")
                self.git("add", ".")
                self.git("commit", "-qm", "changed source")
                self.git("tag", "-am", "mismatched", "bad-" + change)
                self.git("checkout", "-q", self.candidate, "--", "src")
                if change == "inventory":
                    path.unlink()
                with self.assertRaisesRegex(verify.VerificationError, "tag source"):
                    self.verify("bad-" + change)

    def test_source_inventory_limits(self):
        with patch.object(verify, "MAX_FILES", 1):
            with self.assertRaises(verify.VerificationError):
                self.verify()
        with patch.object(verify, "MAX_DEPTH", 0):
            with self.assertRaises(verify.VerificationError):
                self.verify()
        with patch.object(verify, "MAX_TOTAL", 1):
            with self.assertRaises(verify.VerificationError):
                self.verify()

    def test_git_output_limit(self):
        with self.assertRaisesRegex(verify.VerificationError, "git output size limit"):
            verify.git(self.repo, "show", self.candidate + ":src/lib.rs", limit=1)


if __name__ == "__main__":
    unittest.main()
