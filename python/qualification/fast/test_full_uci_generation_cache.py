"""Fail-closed reuse controls; these fixtures never stand in for Sleet tests."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
DIRECTORY = ROOT / 'python/oms-lacal-runtime/tests'
sys.path.insert(0, str(DIRECTORY))
try:
    spec = importlib.util.spec_from_file_location('full_uci_cache_controls', DIRECTORY / 'full_uci_generation.py')
    cache = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cache)
finally:
    sys.path.remove(str(DIRECTORY))


class FullUciGenerationCache(unittest.TestCase):
    def setUp(self):
        self.temporary = TemporaryDirectory(prefix='py016-cache-controls-')
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.source = self.base / 'source'
        subprocess.run(['git', 'init', '-q', str(self.source)], check=True)
        (self.source / 'identity').write_text('source\n')
        subprocess.run(['git', '-C', str(self.source), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.source), '-c', 'user.name=Cache Test',
                        '-c', 'user.email=cache@example.invalid', 'commit', '-qm', 'fixture'], check=True)
        self.sha = subprocess.check_output(['git', '-C', str(self.source), 'rev-parse', 'HEAD'], text=True).strip()
        self.generator = self.base / 'generator'
        self.generator.write_text('#!' + sys.executable + '\n'
            'from pathlib import Path\nimport sys\n'
            'p=Path(sys.argv[-1]); p.mkdir()\n'
            'for n in ["model.py", "service_api.py", "service_codec.py"]:\n'
            ' (p/n).write_text("# generated fixture\\n")\n')
        self.generator.chmod(0o755)
        self.schema = self.base / 'root.xsd'
        self.include = self.base / 'UCI_SecurityMarkings_v2_5_0.xsd'
        self.include.write_text('<schema/>')
        self.schema.write_text('<schema xmlns="http://www.w3.org/2001/XMLSchema">'
                              '<include schemaLocation="' + self.include.name + '"/></schema>')
        self.contract = self.base / 'service.yaml'
        self.contract.write_text('fixture contract\n')
        self.directory = self.base / 'cache'
        self.directory.mkdir()
        self.slot = self.directory / '25-closed-schema'
        for context in (patch.object(cache, 'ROOT', self.source),
                        patch.dict(cache.pins.PINS, {'25': cache.digest(self.schema)}),
                        patch.dict(cache.pins.INCLUDES, {'25': cache.digest(self.include)}),
                        patch.dict(os.environ, AMS_GRA_PY016_GENERATION_CACHE=str(self.directory),
                                   GITHUB_SHA=self.sha, GITHUB_RUN_ID='fixture', GITHUB_RUN_ATTEMPT='1')):
            context.start()
            self.addCleanup(context.stop)

    def generate(self, name='output', release='2.5', world='closed-schema'):
        return cache.generate(self.generator, self.schema, self.contract, release, world, self.base / name)

    def test_identical_inputs_reuse_output_not_execution(self):
        with patch.object(cache.subprocess, 'run', wraps=subprocess.run) as run:
            first = self.generate()
            second = self.generate('again')
        self.assertEqual(sum(call.args[0][0] == str(self.generator)
                             for call in run.call_args_list), 1)
        self.assertEqual((first['cache'], second['cache']), ('miss', 'hit'))
        self.assertEqual(first['identity'], second['identity'])
        self.assertEqual(cache.inventory(self.base / 'output'), cache.inventory(self.base / 'again'))

    def test_changed_generator_rejected(self):
        self.generate()
        self.generator.write_text(self.generator.read_text() + '# changed\n')
        with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
            self.generate('again')

    def test_changed_contract_rejected(self):
        self.generate()
        self.contract.write_text('different contract')
        with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
            self.generate('again')

    def test_wrong_root_and_include_rejected(self):
        for path in (self.schema, self.include):
            with self.subTest(path=path.name):
                original = path.read_bytes()
                path.write_bytes(original + b' ')
                with self.assertRaisesRegex(ValueError, 'wrong pinned'):
                    self.generate()
                path.write_bytes(original)

    def test_unregistered_include_rejected_even_with_matching_root_hash(self):
        self.schema.write_text('<schema xmlns="http://www.w3.org/2001/XMLSchema">'
                              '<import schemaLocation="unexpected.xsd"/></schema>')
        with patch.dict(cache.pins.PINS, {'25': cache.digest(self.schema)}):
            with self.assertRaisesRegex(ValueError, 'wrong pinned'):
                self.generate()

    def test_changed_source_tree_rejected(self):
        self.generate()
        (self.source / 'identity').write_text('new source\n')
        subprocess.run(['git', '-C', str(self.source), 'commit', '-qam', 'changed',
                        '--author=Cache Test <cache@example.invalid>'],
                       env=dict(os.environ, GIT_COMMITTER_NAME='Cache Test',
                                GIT_COMMITTER_EMAIL='cache@example.invalid'), check=True)
        sha = subprocess.check_output(['git', '-C', str(self.source), 'rev-parse', 'HEAD'], text=True).strip()
        with patch.dict(os.environ, GITHUB_SHA=sha):
            with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
                self.generate('again')

    def test_dirty_and_wrong_source_rejected(self):
        with patch.dict(os.environ, GITHUB_SHA='0' * 40):
            with self.assertRaisesRegex(ValueError, 'dirty/mismatched'):
                self.generate()
        (self.source / 'untracked').write_text('dirty')
        with self.assertRaisesRegex(ValueError, 'dirty/mismatched'):
            self.generate()

    def test_cross_run_and_attempt_rejected(self):
        self.generate()
        for key in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT'):
            with self.subTest(key=key), patch.dict(os.environ, {key: 'other'}):
                with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
                    self.generate('again')

    def test_world_and_release_are_separate_selections(self):
        first = self.generate()
        second = self.generate('open', world='open-extensions')
        self.assertNotEqual(first['identity']['world'], second['identity']['world'])
        with self.assertRaisesRegex(ValueError, 'unregistered'):
            self.generate('invalid', release='2.7')
        with self.assertRaisesRegex(ValueError, 'unregistered'):
            self.generate('invalid', world='anything')

    def test_missing_and_changed_generated_files_rejected(self):
        self.generate()
        for name in ('model.py', 'service_api.py', 'service_codec.py'):
            with self.subTest(module=name):
                path = self.slot / 'generated' / name
                original = path.read_bytes()
                path.unlink()
                with self.assertRaisesRegex(ValueError, 'incomplete generated'):
                    self.generate('again')
                path.write_bytes(original + b'# changed')
                with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
                    self.generate('again')
                path.write_bytes(original)

    def test_redirected_generated_files_rejected(self):
        self.generate()
        path = self.slot / 'generated/model.py'
        path.unlink()
        path.symlink_to(self.slot / 'generated/service_api.py')
        with self.assertRaisesRegex(ValueError, 'redirected/nonregular'):
            self.generate('again')

    def test_missing_or_changed_manifest_rejected(self):
        self.generate()
        path = self.slot / 'manifest.json'
        manifest = json.loads(path.read_text())
        manifest['identity']['with_codec'] = False
        path.write_text(json.dumps(manifest))
        with self.assertRaisesRegex(ValueError, 'identity/content mismatch'):
            self.generate('again')
        path.unlink()
        with self.assertRaisesRegex(ValueError, 'incomplete/redirected'):
            self.generate('again')

    def test_failed_generation_not_published(self):
        self.generator.write_text('#!' + sys.executable + '\nraise SystemExit(23)\n')
        with self.assertRaises(subprocess.CalledProcessError):
            self.generate()
        self.assertEqual(list(self.directory.iterdir()), [])

    def test_inputs_changed_during_generation_not_published(self):
        actual = subprocess.run

        def change(*args, **kwargs):
            result = actual(*args, **kwargs)
            if args[0][0] == str(self.generator):
                self.contract.write_text('changed while generating')
            return result

        with patch.object(cache.subprocess, 'run', side_effect=change):
            with self.assertRaisesRegex(ValueError, 'inputs changed'):
                self.generate()
        self.assertEqual(list(self.directory.iterdir()), [])

    def test_absent_cache_retains_real_generation(self):
        with patch.dict(os.environ, AMS_GRA_PY016_GENERATION_CACHE=''):
            self.assertEqual(self.generate()['cache'], 'disabled')
        self.assertFalse(self.slot.exists())

    def test_namespace_package_without_init_is_complete(self):
        self.generate()
        self.assertFalse((self.base / 'output/__init__.py').exists())
        self.assertEqual(set(cache.inventory(self.base / 'output')),
                         {'model.py', 'service_api.py', 'service_codec.py'})

    def test_uncached_miss_and_hit_have_identical_source_hashes(self):
        with patch.dict(os.environ, AMS_GRA_PY016_GENERATION_CACHE=''):
            self.generate('original')
        self.generate('miss')
        self.generate('hit')
        original = cache.inventory(self.base / 'original')
        self.assertEqual(cache.inventory(self.base / 'miss'), original)
        self.assertEqual(cache.inventory(self.base / 'hit'), original)
        manifest = json.loads((self.slot / 'manifest.json').read_text())
        self.assertEqual(manifest['files'], original)

    def test_corrupted_and_wrong_shape_manifests_rejected(self):
        self.generate()
        path = self.slot / 'manifest.json'
        for text in ('', '{', 'null', '[]', '{}', '{"identity": {}}'):
            with self.subTest(manifest=text):
                path.write_text(text)
                with self.assertRaises(ValueError):
                    self.generate('again')

    def test_incomplete_successful_generation_never_published(self):
        original = self.generator.read_text()
        for name in ('model.py', 'service_api.py', 'service_codec.py'):
            with self.subTest(module=name):
                self.generator.write_text(original + f'(p/{name!r}).unlink()\n')
                with self.assertRaisesRegex(ValueError, 'incomplete generated'):
                    self.generate()
                self.assertEqual(list(self.directory.iterdir()), [])
                self.assertFalse((self.base / 'output').exists())

    def test_distinct_pinned_releases_never_share_output(self):
        first = self.generate()
        include26 = self.base / 'UCI_SecurityMarkings_v2_6_0.xsd'
        include26.write_text('<schema version="2.6"/>')
        self.schema.write_text('<schema xmlns="http://www.w3.org/2001/XMLSchema">'
                              '<include schemaLocation="' + include26.name + '"/></schema>')
        with patch.dict(cache.pins.PINS, {'26': cache.digest(self.schema)}), \
                patch.dict(cache.pins.INCLUDES, {'26': cache.digest(include26)}):
            second = self.generate('release26', release='2.6')
            self.assertEqual(second['cache'], 'miss')
            self.assertNotEqual(first['identity']['release'], second['identity']['release'])
            self.assertNotEqual(first['identity']['schema_sha256'], second['identity']['schema_sha256'])
            self.assertEqual(self.generate('release26hit', release='2.6')['cache'], 'hit')
        self.assertTrue(self.slot.is_dir())
        self.assertTrue((self.directory / '26-closed-schema').is_dir())

    def test_nonregular_generated_input_rejected(self):
        self.generate()
        path = self.slot / 'generated/model.py'
        path.unlink()
        os.mkfifo(path)
        with self.assertRaisesRegex(ValueError, 'redirected/nonregular'):
            self.generate('again')

    def test_redirected_and_nonregular_manifests_rejected(self):
        self.generate()
        path = self.slot / 'manifest.json'
        original = self.base / 'manifest.json'
        path.rename(original)
        path.symlink_to(original)
        with self.assertRaisesRegex(ValueError, 'incomplete/redirected'):
            self.generate('again')
        path.unlink()
        os.mkfifo(path)
        with self.assertRaisesRegex(ValueError, 'incomplete/redirected'):
            self.generate('again')


if __name__ == '__main__':
    unittest.main()