"""Installer contracts using local download fixtures; no network requests."""
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest


INSTALLER = Path(__file__).resolve().parents[1] / 'install.sh'


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.home = self.root / 'home'
        self.home.mkdir()
        self.shims = self.root / 'shims'
        self.shims.mkdir()
        self.asset = 'ontodoc-0.1.2-linux-x86_64-gnu.tar.gz'
        payload = b'#!/bin/sh\nprintf "ontodoc 0.1.2\\n"\n'
        with tarfile.open(self.root / self.asset, 'w:gz') as archive:
            info = tarfile.TarInfo('ontodoc-0.1.2/bin/ontodoc')
            info.size = len(payload)
            info.mode = 0o755
            archive.addfile(info, io.BytesIO(payload))
        digest = hashlib.sha256((self.root / self.asset).read_bytes()).hexdigest()
        (self.root / 'SHA256SUMS').write_text(digest + '  ' + self.asset + '\n')
        self.script('getconf', '#!/bin/sh\necho "glibc 2.39"\n')
        self.script('uname', '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo x86_64;; esac\n')
        self.script('curl', '''#!/usr/bin/env python3
import os, pathlib, shutil, sys
root = pathlib.Path(os.environ['FIXTURE'])
args = sys.argv[1:]
with (root / 'requests').open('a') as log:
    log.write(args[-1] + '\\n')
if os.environ.get('DOWNLOAD_FAIL'):
    sys.exit(22)
assert args[-1].startswith('https://github.com/ontograph/odoc/releases/')
if args[-1].endswith('/latest'):
    print('https://github.com/ontograph/odoc/releases/tag/v0.1.2', end='')
else:
    shutil.copyfile(root / args[-1].rsplit('/', 1)[-1], args[args.index('--output') + 1])
''')
        self.env = {**os.environ, 'HOME':str(self.home), 'FIXTURE':str(self.root),
                    'PATH':str(self.shims) + os.pathsep + os.environ['PATH'],
                    'TMPDIR':str(self.root)}
        self.destination = self.home / '.local/bin/ontodoc'

    def script(self, name, text):
        path = self.shims / name
        path.write_text(text)
        path.chmod(0o755)

    def invoke(self, *args, success=True):
        result = subprocess.run(['sh', str(INSTALLER), *args], env=self.env,
                                text=True, capture_output=True, timeout=10)
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(list(self.root.glob('tmp*')), 'temporary downloads must be removed')
        return result

    def test_latest_install_and_idempotent_reinstall(self):
        self.invoke()
        self.assertEqual(subprocess.check_output([str(self.destination), '--version'], text=True), 'ontodoc 0.1.2\n')
        self.invoke('--version', 'v0.1.2')
        self.assertFalse(list(self.destination.parent.glob('ontodoc.bak-*')))
        self.assertIn('/releases/latest', (self.root / 'requests').read_text())

    def test_custom_directory_with_spaces(self):
        target = self.root / 'custom bin'
        self.invoke('--version', '0.1.2', '--bin-dir', str(target))
        self.assertTrue((target / 'ontodoc').is_file())

    def test_upgrade_backs_up_existing_symlink(self):
        self.destination.parent.mkdir(parents=True)
        old = self.root / 'old'
        old.write_text('previous binary')
        self.destination.symlink_to(old)
        self.invoke('--version', '0.1.2')
        backups = list(self.destination.parent.glob('ontodoc.bak-*'))
        self.assertEqual(len(backups), 1)
        self.assertTrue(backups[0].is_symlink())
        self.assertEqual(old.read_text(), 'previous binary')
        self.assertFalse(self.destination.is_symlink())

    def test_bad_checksum_preserves_existing_binary(self):
        self.destination.parent.mkdir(parents=True)
        self.destination.write_text('untouched')
        (self.root / 'SHA256SUMS').write_text('0' * 64 + '  ' + self.asset + '\n')
        self.invoke('--version', '0.1.2', success=False)
        self.assertEqual(self.destination.read_text(), 'untouched')
        self.assertFalse(list(self.destination.parent.glob('ontodoc.bak-*')))

    def test_missing_and_duplicate_checksums_fail(self):
        original = (self.root / 'SHA256SUMS').read_text()
        for content in ('', original * 2):
            (self.root / 'SHA256SUMS').write_text(content)
            self.invoke('--version', '0.1.2', success=False)
            self.assertFalse(self.destination.exists())

    def test_download_failure_has_no_install(self):
        self.env['DOWNLOAD_FAIL'] = '1'
        self.invoke('--version', '0.1.2', success=False)
        self.assertFalse(self.destination.exists())

    def test_wrong_binary_version_preserves_existing_binary(self):
        self.destination.parent.mkdir(parents=True)
        self.destination.write_text('untouched')
        payload = b'#!/bin/sh\nprintf "ontodoc 0.0.0\\n"\n'
        with tarfile.open(self.root / self.asset, 'w:gz') as archive:
            info = tarfile.TarInfo('ontodoc-0.1.2/bin/ontodoc')
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))
        digest = hashlib.sha256((self.root / self.asset).read_bytes()).hexdigest()
        (self.root / 'SHA256SUMS').write_text(digest + '  ' + self.asset + '\n')
        self.invoke('--version', '0.1.2', success=False)
        self.assertEqual(self.destination.read_text(), 'untouched')

    def test_invalid_arguments_do_not_download(self):
        for args in (('--version', '../bad'), ('--version', '0.1.2\n../bad'), ('--version',), ('--unknown',), ('--bin-dir', 'relative')):
            self.invoke(*args, success=False)
        self.assertFalse((self.root / 'requests').exists())

    def test_platform_and_glibc_refusals(self):
        self.script('uname', '#!/bin/sh\necho unsupported\n')
        self.invoke(success=False)
        self.script('uname', '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo x86_64;; esac\n')
        self.script('getconf', '#!/bin/sh\necho "glibc 2.38"\n')
        self.invoke(success=False)
        self.assertFalse((self.root / 'requests').exists())

    def test_help_has_no_side_effects(self):
        self.assertIn('Usage:', self.invoke('--help').stdout)
        self.assertFalse((self.root / 'requests').exists())


if __name__ == '__main__':
    unittest.main()
