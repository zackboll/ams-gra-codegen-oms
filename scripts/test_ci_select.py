"""Positive and adversarial CI policy controls; real Git history, no API file list."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection as selection

spec = importlib.util.spec_from_file_location('ci_required', selection.ROOT / 'scripts/ci-required-gate.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
report_spec = importlib.util.spec_from_file_location('ci_report', selection.ROOT / 'scripts/ci-concurrency-report.py')
reporter = importlib.util.module_from_spec(report_spec)
report_spec.loader.exec_module(reporter)


def change(path, status='M', mode='100644'):
    return dict(paths=[path], status=status, old_mode='100644', new_mode=mode)


class SelectionTests(unittest.TestCase):
    def setUp(self):
        self.policy = selection.inventory()

    def selected(self, *paths):
        manifest = selection.select([change(p) for p in paths], self.policy)
        return {j for j, enabled in manifest['jobs'].items() if enabled}

    def test_harmless_docs_baseline_only(self):
        self.assertEqual(self.selected('README.md', 'docs/design.md'), set())
        self.assertEqual(selection.select([change('docs/design.md')], self.policy)['selected_cells'], 0)

    def test_executable_and_compiled_markdown_not_harmless(self):
        self.assertEqual(self.selected('docs/task-044-enum-identifier-remapping.md'), {'rust'})
        for path in ('docs/foo.md', 'tests/fixtures/frozen.md', 'scripts/README.md'):
            manifest = selection.select([change(path, mode='100755')], self.policy)
            self.assertEqual(manifest['mode'], 'full-pr')

    def test_python_both_versions_and_shared_cli(self):
        self.assertEqual(self.selected('python/oms-lacal-runtime/src/oms_lacal/client.py'), {'rust','python-model-backend'})
        manifest = selection.select([change('crates/backend-python/src/lib.rs')], self.policy)
        self.assertIn('python-model-backend-3.11', manifest['cells'])
        self.assertIn('python-model-backend-3.13', manifest['cells'])
        self.assertNotIn('ada-float32-codec',manifest['cells'])

    def test_float32_inherited_and_composition_not_python(self):
        selected = self.selected('ada/oms-json-support/float32/ams_gra_float32.c')
        self.assertEqual(selected, {'rust','ada-json-guard','ada-float32-codec','ada-float64-codec','ada-time-zulu-codec','ada-lacal-runtime-core'})
        self.assertNotIn('python-model-backend', selected)

    def test_shared_api_and_dependency_changes_full(self):
        for path in ('crates/codegen-core/src/lib.rs','crates/ir/src/lib.rs','crates/cli/src/main.rs',
                     'Cargo.lock','crates/backend-python/Cargo.toml','rust-toolchain.toml'):
            with self.subTest(path=path):
                self.assertEqual(self.selected(path), set(selection.ordinary_jobs(self.policy)))

    def test_unknown_ci_and_frozen_fixture_fail_broad(self):
        for path in ('unknown.rs','.github/workflows/new.yml','scripts/ci_selection.py',
                     'tests/fixtures/service-generate/codec-ada-float32.xsd'):
            result=selection.select([change(path)],self.policy)
            self.assertEqual(result['mode'],'full-pr')
            self.assertTrue(result['deep'] if not path.startswith('tests/') else True)

    def test_changed_test_owned_and_temporal_fixture_not_python(self):
        self.assertIn('ada-float64-codec', self.selected('tests/ada-float64-codec/conformance.adb'))
        selected = self.selected('tests/fixtures/temporal/duration.txt')
        self.assertTrue({'ada-duration-codec','ada-datetime-codec','ada-repeated-temporal-codec'} <= selected)
        self.assertNotIn('python-model-backend', selected)

    def test_deletions_renames_typechanges_full(self):
        for status in ('D','R100','C100','T','U'):
            c=change('docs/design.md',status)
            if status[0] in 'RC':
                c['paths']=['crates/ir/src/lib.rs','docs/design.md']
            result=selection.select([c],self.policy)
            self.assertEqual(result['mode'],'full-pr')

    def test_prerequisite_closure(self):
        jobs=selection.ordinary_jobs(self.policy)
        self.assertEqual(selection.close_prerequisites({'ada-duration-codec'},jobs), {'ada-duration-codec','ada-json-guard'})
        jobs['ada-duration-codec']['prerequisites']=['not-in-inventory']
        with self.assertRaises((KeyError,ValueError)):
            selection.close_prerequisites({'ada-duration-codec'},jobs)

    def test_main_always_all(self):
        for event in ('push','schedule','workflow_dispatch'):
            manifest=selection.classify('/not-needed',{},event,'f'*40,self.policy)
            self.assertEqual(manifest['mode'],'full-main')
            self.assertTrue(all(manifest['jobs'].values()))
            self.assertTrue(manifest['deep'])

    def test_diff_parser_truncation_malformed_and_rename(self):
        header=b':100644 100644 '+b'a'*40+b' '+b'b'*40+b' R100\0'
        result=selection.parse_diff(header+b'old.rs\0new.rs\0')
        self.assertEqual(result[0]['paths'],['old.rs','new.rs'])
        for raw in (header,header+b'old.rs\0',header+b'old.rs\0new.rs',b'broken\0',header+b'../bad\0new\0'):
            with self.subTest(raw=raw),self.assertRaises(ValueError):
                selection.parse_diff(raw)


class RealGitTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        self.run_git('init','-q','-b','main')
        self.run_git('config','user.name','CI selection test')
        self.run_git('config','user.email','ci@example.invalid')
        self.commit('README.md','base')
        self.initial=self.sha()
        self.run_git('checkout','-qb','feature')
        self.commit('docs/a.md','first')
        self.commit('docs/b.md','second')
        self.head=self.sha()
        self.run_git('checkout','-q','main')
        self.commit('main-only.rs','main advanced')
        self.base=self.sha()
        self.run_git('merge','--no-ff','-qm','synthetic merge','feature')
        self.merge=self.sha()
        self.event=dict(pull_request=dict(base=dict(sha=self.base),head=dict(sha=self.head)))

    def run_git(self,*args):
        return subprocess.check_output(['git','-C',str(self.root),*args],stderr=subprocess.STDOUT).decode().strip()

    def sha(self):
        return self.run_git('rev-parse','HEAD')

    def commit(self,path,text):
        p=self.root/path
        p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(text+'\n')
        self.run_git('add','.')
        self.run_git('commit','-qm',text)

    def classify(self,event=None,sha=None):
        return selection.classify(self.root,event or self.event,'pull_request',sha or self.merge)

    def test_multicommit_and_advanced_main_complete_merge_diff(self):
        manifest=self.classify()
        self.assertEqual(manifest['mode'],'selective-pr')
        self.assertEqual([c['paths'] for c in manifest['changes']],[['docs/a.md'],['docs/b.md']])

    def test_merge_incorporated_from_main(self):
        self.run_git('checkout','-q','feature')
        self.run_git('merge','-qm','incorporate main',self.base)
        self.head=self.sha()
        self.run_git('checkout','-q','main')
        self.run_git('reset','--hard',self.base)
        self.commit('another-main.rs','new main')
        self.base=self.sha()
        self.run_git('merge','--no-ff','-qm','synthetic merge','feature')
        self.merge=self.sha()
        self.event['pull_request']['base']['sha']=self.base
        self.event['pull_request']['head']['sha']=self.head
        self.assertEqual(self.classify()['selected_cells'],0)

    def test_missing_mismatched_objects_and_checkout_fail_full(self):
        for key,sha in (('base','0'*40),('base',self.initial),('head',self.base),('head','missing')):
            event=copy.deepcopy(self.event)
            event['pull_request'][key]['sha']=sha
            manifest=self.classify(event)
            self.assertEqual(manifest['mode'],'full-pr')
            self.assertTrue(manifest['reasons'][0]['reason'])
        self.assertEqual(self.classify(sha=self.head)['mode'],'full-pr')

    def test_shallow_and_failed_diff_fail_full(self):
        original=selection.git
        for failure in ('shallow','diff'):
            def broken(root,*args):
                if failure=='shallow' and args==('rev-parse','--is-shallow-repository'):
                    return b'true\n'
                if failure=='diff' and args[0]=='diff':
                    raise ValueError('diff collection failed')
                return original(root,*args)
            with patch.object(selection,'git',side_effect=broken):
                self.assertEqual(self.classify()['mode'],'full-pr')

    def test_actual_deleted_and_renamed_files(self):
        for operation in ('delete','rename'):
            self.run_git('checkout','-q','feature')
            self.run_git('reset','--hard',self.head)
            if operation=='delete':
                self.run_git('rm','README.md')
            else:
                self.run_git('mv','README.md','renamed.md')
            self.run_git('commit','-qm',operation)
            changed_head=self.sha()
            self.run_git('checkout','-q','main')
            self.run_git('reset','--hard',self.base)
            self.run_git('merge','--no-ff','-qm','synthetic '+operation,'feature')
            event=copy.deepcopy(self.event)
            event['pull_request']['head']['sha']=changed_head
            self.assertEqual(self.classify(event,self.sha())['mode'],'full-pr')

    def test_truncation_at_complete_record_boundary_detected(self):
        original=selection.git
        def truncated(root,*args):
            result=original(root,*args)
            if args[:2]==('diff','--raw'):
                # Keep one complete record from the real two-file diff.
                result=b'\0'.join(result.split(b'\0')[:2])+b'\0'
            return result
        with patch.object(selection,'git',side_effect=truncated):
            manifest=self.classify()
            self.assertEqual(manifest['mode'],'full-pr')
            self.assertIn('independent path inventories disagree',manifest['reasons'][0]['reason'])


class GateTests(unittest.TestCase):
    def setUp(self):
        self.policy=selection.inventory()
        self.manifest=selection.select([],self.policy,full_reason='negative control')
        self.needs={j:dict(result='success') for j in self.manifest['jobs']}
        self.needs.update({j:dict(result='success') for j in ('changes','baseline','deep-qualification')})
        self.jobs=[]
        self.artifacts=[]
        for record in self.policy['jobs']:
            if record['workflow'] not in ('ci.yml', 'deep-ci.yml'):
                continue
            for cell in record['cells']:
                self.jobs.append(dict(name=('deep-qualification / ' if record['workflow']=='deep-ci.yml' else '')+cell['name'],
                    status='completed',conclusion='success',started_at='2026-10-27T01:00:00Z',
                    steps=[dict(name=s.get('name','Run command'),conclusion='success') for s in record['original_steps']]))
                self.artifacts.extend(dict(name=n,expired=False,size_in_bytes=1,created_at='2026-10-27T02:00:00Z') for n in cell['artifacts'])

    def validate(self):
        gate.validate(self.manifest,self.manifest,self.needs,self.jobs,self.artifacts,self.policy)

    def test_healthy_full_and_docs(self):
        self.validate()
        self.manifest=selection.select([change('README.md')],self.policy)
        self.needs.update({j:dict(result='skipped') for j in self.manifest['jobs']})
        self.needs['deep-qualification']['result']='skipped'
        self.validate()

    def test_selected_failure_cancellation_skipped_missing_conclusion(self):
        for result in ('failure','cancelled','skipped',None):
            for target in ('rust','ada-json-guard','deep-qualification','baseline','changes'):
                with self.subTest(result=result,target=target):
                    saved=self.needs[target]['result']
                    self.needs[target]['result']=result
                    with self.assertRaises(ValueError):
                        self.validate()
                    self.needs[target]['result']=saved

    def test_matrix_cell_hidden_failure_or_never_started(self):
        for status,conclusion in (('completed','failure'),('completed','cancelled'),('queued',None),('completed','skipped')):
            self.jobs[0].update(status=status,conclusion=conclusion)
            with self.assertRaises(ValueError):
                self.validate()
        self.jobs.pop(0)
        with self.assertRaises(ValueError):
            self.validate()

    def test_missing_evidence_and_skipped_qualification_step(self):
        artifact=self.artifacts.pop()
        with self.assertRaises(ValueError):
            self.validate()
        self.artifacts.append(artifact)
        self.jobs[0]['steps'][0]['conclusion']='skipped'
        with self.assertRaises(ValueError):
            self.validate()

    def test_bad_exclusion_missing_prerequisite_rejected(self):
        expected=copy.deepcopy(self.manifest)
        self.manifest['jobs']['ada-json-guard']=False
        with self.assertRaisesRegex(ValueError,'independently recomputed'):
            gate.validate(self.manifest,expected,self.needs,self.jobs,self.artifacts,self.policy)
        self.needs['ada-json-guard']['result']='skipped'
        with self.assertRaises(ValueError):
            self.validate()


class SchedulingTests(unittest.TestCase):
    def setUp(self):
        self.policy=selection.inventory()
        self.workflows={p.name:p.read_text() for p in (selection.ROOT/'.github/workflows').glob('*.yml')}

    def check(self):
        return ci_policy.check(self.policy,self.workflows)

    def test_complete_36_runner_cells_five_common_lanes(self):
        self.assertEqual(self.check(),36)

    def test_six_lanes_duplicate_invalid_or_unassigned_rejected(self):
        for lanes in ([f'oms-main-slot-{n}' for n in range(1,7)],['oms-main-slot-1']*5):
            self.policy['lanes']=lanes
            with self.assertRaises(ValueError):
                self.check()
        self.policy=selection.inventory()
        for value in (6,None,0,'1'):
            self.policy['jobs'][0]['cells'][0]['lane']=value
            with self.assertRaises(ValueError):
                self.check()

    def test_bypass_queue_pool_matrix_main_cancellation_pages(self):
        original=copy.deepcopy(self.workflows)
        cases=[('ci.yml','      queue: max','      queue: single'),
               ('ci.yml','      queue: max','      queue: max\n      cancel-in-progress: true'),
               ('ci.yml',"'oms-main-slot-5'","'separate-main-slot-6'"),
               ('ci.yml','            slot: 1','            missing: 1'),
               ('ci.yml',"cancel-in-progress: ${{ github.event_name == 'pull_request' }}",'cancel-in-progress: true'),
               ('uci-pages.yml','      queue: max','      queue: single'),
               ('deep-ci.yml','    concurrency:','    bypassed-concurrency:'),
               ('ci.yml','    if: ${{ always() }}','    if: success()')]
        for workflow,before,after in cases:
            self.workflows=copy.deepcopy(original)
            self.workflows[workflow]=self.workflows[workflow].replace(before,after,1)
            with self.subTest(after=after),self.assertRaises(ValueError):
                self.check()

    def test_added_job_new_workflow_and_matrix_rejected(self):
        original=copy.deepcopy(self.workflows)
        self.workflows['ci.yml']+='\n  surprise:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n'
        with self.assertRaises(ValueError):
            self.check()
        self.workflows=copy.deepcopy(original)
        self.workflows['sixth-pool.yml']='jobs:\n  surprise:\n    runs-on: ubuntu-latest\n'
        with self.assertRaises(ValueError):
            self.check()

    def test_original_commands_markers_and_compilers_pinned(self):
        for before,after in [('TASK095_BRIDGE_CONTROLS_COMPLETE checks=20','TASK095_BRIDGE_CONTROLS_COMPLETE checks=0'),
                             ("compiler: ['14', '16']","compiler: ['14']"),('cargo test --workspace','cargo test -p ams-gra-oms-ir')]:
            original=self.workflows['ci.yml']
            self.workflows['ci.yml']=original.replace(before,after)
            with self.subTest(before=before),self.assertRaises(ValueError):
                self.check()
            self.workflows['ci.yml']=original


class TimestampTests(unittest.TestCase):
    def test_five_allowed_six_rejected_and_pending_not_execution(self):
        jobs=[dict(id=n,name=f'pilot-{n}',status='completed',conclusion='success',
                   started_at='2026-10-27T00:00:00Z',completed_at='2026-10-27T00:10:30Z',
                   steps=[dict(started_at='2026-10-27T00:10:00Z',conclusion='success')]) for n in range(6)]
        report=reporter.report(jobs[:5])
        self.assertEqual(report['maximum_running'],5)
        self.assertEqual(report['runner_minutes'],2.5)
        self.assertEqual(report['wall_seconds'],30)
        with self.assertRaises(ValueError):
            reporter.report(jobs)
        jobs[0]['status']='pending'
        with self.assertRaises(ValueError):
            reporter.report(jobs[:1])

    def test_half_open_boundaries_not_double_counted(self):
        jobs=[dict(id=1,name='a',status='completed',conclusion='success',completed_at='2026-10-27T00:10:30Z',
                   steps=[dict(started_at='2026-10-27T00:10:00Z',conclusion='success')]),
              dict(id=2,name='b',status='completed',conclusion='success',completed_at='2026-10-27T00:11:00Z',
                   steps=[dict(started_at='2026-10-27T00:10:30Z',conclusion='success')])]
        self.assertEqual(reporter.report(jobs,1)['maximum_running'],1)


if __name__=='__main__':
    unittest.main()