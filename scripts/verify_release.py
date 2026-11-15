"""Read-only verification of the preserved 0.1.0 package and scoped source bytes.

This checks identities, not reproducible builds, historical smoke runs or release
authorization. It never extracts the archive or executes its binary.
Run against a quiescent trusted workspace; this is not a concurrent filesystem
snapshot or a race-containment boundary.
"""

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import selectors
import stat
import subprocess
import tarfile
import time


CANDIDATE = "cc116c44a4d7f854bc19c6a4b40d75098a437268"
BINARY_SHA = "af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55"
ARCHIVE_SHA = "e53b9453cbf0c85f8f6f5ab7fbee7e45f1fc92933420c9f0800a1d8299f24dd6"
NAME = "ontodoc-0.1.0-linux-x86_64-gnu"
MEMBERS = frozenset(("CHANGELOG.md", "DEPLOYMENT.md", "LICENSE", "LICENSE-APACHE",
                     "NOTICE", "README.md", "SHA256SUMS", "THIRD_PARTY_LICENSES.md",
                     "bin/ontodoc"))
ROOT_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
MAX_RECEIPT = 128 * 1024
MAX_ARCHIVE = 32 * 1024 * 1024
MAX_MEMBER = 32 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024
MAX_FILES = 4096
MAX_DEPTH = 64


class VerificationError(ValueError):
    """An identity, layout or resource limit check failed."""


def require(condition, message):
    if not condition:
        raise VerificationError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read_regular(path, limit):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode), f"not a regular file: {path}")
        require(info.st_size <= limit, f"file size limit: {path}")
        data = stream.read(limit + 1)
    require(len(data) <= limit, f"file size limit: {path}")
    return data


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate receipt key")
        result[key] = value
    return result


def checksums(data, expected):
    require(len(data) <= MAX_RECEIPT, "checksum size limit")
    result = {}
    for line in data.decode("ascii").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([^\r\n]+)", line)
        require(match is not None, "malformed checksum line")
        digest, name = match.groups()
        require(name not in result, "duplicate checksum name")
        result[name] = digest
    require(result.keys() == expected, "checksum coverage mismatch")
    return result


def archive_members(data):
    # Decompress before interpreting tar metadata: even hidden PAX/GNU headers
    # must fit this budget. Parsing raw headers lets us refuse those specials.
    with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
        raw = compressed.read(MAX_TOTAL + 1)
    require(len(raw) <= MAX_TOTAL, "decompressed archive size limit")
    offset = 0
    members = {}
    total = 0
    while offset + 512 <= len(raw):
        header = raw[offset:offset + 512]
        if header == bytes(512):
            require(len(raw) - offset >= 1024 and not any(raw[offset:]),
                    "invalid tar end marker or trailing data")
            break
        item = tarfile.TarInfo.frombuf(header, "utf-8", "strict")
        require(item.type in (tarfile.REGTYPE, tarfile.AREGTYPE),
                "archive contains a link or special member")
        require(item.name.startswith(NAME + "/"), "archive prefix mismatch")
        name = item.name[len(NAME) + 1:]
        require(name in MEMBERS, "unexpected archive member")
        require(name not in members, "duplicate archive member")
        require(0 <= item.size <= MAX_MEMBER, "archive member size limit")
        total += item.size
        require(total <= MAX_TOTAL, "archive total size limit")
        start = offset + 512
        end = start + item.size
        offset = start + ((item.size + 511) // 512) * 512
        require(offset <= len(raw), "truncated archive member")
        members[name] = (raw[start:end], item.mode)
    else:
        raise VerificationError("missing tar end marker")
    require(members.keys() == MEMBERS, "archive member coverage mismatch")
    return members


def verify_package(package, *, candidate=CANDIDATE, binary_sha=BINARY_SHA,
                   archive_sha=ARCHIVE_SHA):
    receipt = json.loads(read_regular(package / "validation.json", MAX_RECEIPT),
                         object_pairs_hook=unique_object)
    require(isinstance(receipt, dict), "receipt must be an object")
    archive_name = NAME + ".tar.gz"
    for key, value in (("archive", archive_name), ("candidate_commit", candidate),
                       ("binary_sha256", binary_sha), ("archive_sha256", archive_sha)):
        require(receipt.get(key) == value, f"receipt identity mismatch: {key}")
    records = receipt.get("files")
    require(isinstance(records, dict) and records.keys() == MEMBERS,
            "receipt member coverage mismatch")
    external = checksums(read_regular(package / "SHA256SUMS", MAX_RECEIPT), {archive_name})
    require(external[archive_name] == archive_sha, "external archive checksum mismatch")
    data = read_regular(package / archive_name, MAX_ARCHIVE)
    require(sha(data) == archive_sha, "archive hash mismatch")
    members = archive_members(data)
    digests = {}
    for name, (content, mode) in members.items():
        record = records[name]
        require(isinstance(record, dict), "invalid receipt member record")
        require(type(record.get("bytes")) is int and record["bytes"] == len(content),
                f"receipt member size mismatch: {name}")
        digests[name] = sha(content)
        require(record.get("sha256") == digests[name], f"receipt member hash mismatch: {name}")
        if name == "bin/ontodoc":
            require(mode & 0o111 != 0, "binary is not executable")
            require(digests[name] == binary_sha, "binary hash mismatch")
    internal = checksums(members["SHA256SUMS"][0], MEMBERS - {"SHA256SUMS"})
    require(all(digests[name] == digest for name, digest in internal.items()),
            "internal member checksum mismatch")
    return {"archive_sha256": archive_sha, "binary_sha256": binary_sha,
            "regular_members": len(members)}


def git(repo, *arguments, limit=MAX_MEMBER):
    environment = dict(os.environ, GIT_NO_LAZY_FETCH="1", GIT_OPTIONAL_LOCKS="0",
                       GIT_TERMINAL_PROMPT="0")
    command = ["git", "--no-replace-objects", "-C", str(repo), *arguments]
    # Bound output while reading, and kill/reap Git on cap, failure or timeout.
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                          env=environment) as process:
        try:
            output = bytearray()
            deadline = time.monotonic() + 30
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                while True:
                    remaining = deadline - time.monotonic()
                    require(remaining > 0, "git command timeout")
                    require(selector.select(remaining), "git command timeout")
                    chunk = os.read(process.stdout.fileno(), min(65536, limit + 1 - len(output)))
                    if not chunk:
                        break
                    output.extend(chunk)
                    require(len(output) <= limit, "git output size limit")
            require(process.wait(timeout=max(0.001, deadline - time.monotonic())) == 0,
                    "git command failed")
            return bytes(output)
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()


def git_inventory(repo, commit):
    require(re.fullmatch(r"[0-9a-f]{40}", commit) is not None, "invalid commit identity")
    raw = git(repo, "ls-tree", "-rz", "--full-tree", commit, "--", "src", *ROOT_FILES,
              limit=MAX_RECEIPT * 8)
    result = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        metadata, name = record.split(b"\t", 1)
        mode, kind, _object_id = metadata.split(b" ")
        path = os.fsdecode(name)
        require(mode in (b"100644", b"100755") and kind == b"blob",
                "candidate contains symlink or special source entry")
        require(path.startswith("src/") or path in ROOT_FILES, "invalid candidate source path")
        require(path not in result, "duplicate candidate source path")
        result[path] = None
        require(len(result) <= MAX_FILES, "source file count limit")
    require(all(path in result for path in ROOT_FILES), "candidate missing root source file")
    return result


def workspace_inventory(repo):
    result = {}
    entries = 0

    def walk(descriptor, prefix, depth):
        nonlocal entries
        require(depth <= MAX_DEPTH, "source depth limit")
        # Charge entries before retaining or sorting the directory inventory.
        with os.scandir(descriptor) as iterator:
            names = []
            for entry in iterator:
                entries += 1
                require(entries <= MAX_FILES, "source entry count limit")
                names.append(entry.name)
        for name in sorted(names):
            path = prefix + "/" + name
            info = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
            if stat.S_ISDIR(info.st_mode):
                child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                dir_fd=descriptor)
                try:
                    walk(child, path, depth + 1)
                finally:
                    os.close(child)
            else:
                require(stat.S_ISREG(info.st_mode), f"non-regular source entry: {path}")
                result[path] = None

    source = os.open(repo / "src", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        walk(source, "src", 0)
    finally:
        os.close(source)
    for path in ROOT_FILES:
        require(stat.S_ISREG((repo / path).lstat().st_mode), f"non-regular source entry: {path}")
        result[path] = None
    return result


def verify_sources(repo, *, candidate=CANDIDATE, tag=None):
    expected = git_inventory(repo, candidate)
    actual = workspace_inventory(repo)
    require(actual.keys() == expected.keys(), "workspace source inventory mismatch")
    total = 0
    for path in sorted(expected):
        content = git(repo, "show", f"{candidate}:{path}")
        total += len(content)
        require(total <= MAX_TOTAL, "source total size limit")
        require(read_regular(repo / path, min(MAX_MEMBER, MAX_TOTAL - total + len(content))) == content,
                f"workspace source bytes mismatch: {path}")
    result = {"candidate_commit": candidate, "regular_files": len(expected)}
    if tag is not None:
        require(tag and not tag.startswith("refs/"), "tag must be a literal short tag name")
        reference = "refs/tags/" + tag
        git(repo, "check-ref-format", reference, limit=1024)
        require(git(repo, "cat-file", "-t", reference, limit=128).strip() == b"tag",
                "tag must be annotated")
        commit = git(repo, "rev-parse", "--verify", reference + "^{commit}", limit=128).decode("ascii").strip()
        require(git_inventory(repo, commit).keys() == expected.keys(), "tag source inventory mismatch")
        total = 0
        for path in sorted(expected):
            content = git(repo, "show", f"{commit}:{path}")
            total += len(content)
            require(total <= MAX_TOTAL, "tag source total size limit")
            require(content == git(repo, "show", f"{candidate}:{path}"),
                    f"tag source bytes mismatch: {path}")
        result["annotated_tag"] = tag
        result["tag_commit"] = commit
    return result


def main():
    repo = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, default=repo / "tmp/release-010/package")
    parser.add_argument("--tag", help="literal annotated tag name; scoped source bytes must match candidate")
    args = parser.parse_args()
    skipped = ["build and binary execution", "historical smoke rerun", "publication authorization"]
    if args.tag is None:
        skipped.append("annotated tag (no --tag supplied)")
    result = {"success": False, "scope": "archive_and_source_identity", "checks": {}, "skipped": skipped}
    try:
        result["checks"]["package"] = verify_package(args.package)
        result["checks"]["source"] = verify_sources(repo, tag=args.tag)
        result["success"] = True
    except (VerificationError, OSError, ValueError, RecursionError, tarfile.TarError, EOFError,
            subprocess.SubprocessError) as error:
        result["error"] = str(error)[:240]
    print(json.dumps(result, sort_keys=True))
    return 0 if result["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
