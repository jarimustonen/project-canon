"""Regression gate for the approved cargo-dist macOS CI override.

Run with `python3 tests/release_workflow.py` (requires PyYAML). cargo-dist's
allow-dirty = ["ci"] makes its own generate --check blind to these edits.
"""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('release_workflow_guard', ROOT / 'scripts/release_workflow.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
ORIGINAL, REPLACEMENT, check = module.ORIGINAL, module.REPLACEMENT, module.check
WORKFLOW = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())


class ReleaseWorkflowTest(unittest.TestCase):
    def test_approved_scope(self):
        jobs = WORKFLOW["jobs"]
        plan = jobs["plan"]["steps"]
        self.assertEqual(plan[1]["name"], "Install dist")
        self.assertEqual(plan[2]["with"]["path"], "~/.cargo/bin/dist")
        local = jobs["build-local-artifacts"]["steps"]
        mac = next(s for s in local if s.get("name") == "Install dist (self-hosted macOS)")
        linux = next(s for s in local if s.get("name") == "Install dist (other local runners)")
        self.assertEqual(mac["if"], "${{ matrix.runner == 'self-hosted' && runner.os == 'macOS' }}")
        self.assertEqual(mac["shell"], "bash")
        self.assertIn('CARGO_DIST_NO_MODIFY_PATH=1', mac['run'])
        self.assertIn('CARGO_DIST_INSTALL_DIR=', mac['run'])
        self.assertIn('${{ matrix.install_dist.run }}', mac['run'])
        self.assertIn('test "$(command -v dist)" = "$CARGO_DIST_INSTALL_DIR/bin/dist"', mac['run'])
        config = (ROOT / 'dist-workspace.toml').read_text()
        self.assertIn('aarch64-apple-darwin = "self-hosted"', config)
        self.assertIn('allow-dirty = ["ci"]', config)
        self.assertEqual(linux["if"], "${{ !(matrix.runner == 'self-hosted' && runner.os == 'macOS') }}")
        self.assertEqual(linux["run"], "${{ matrix.install_dist.run }}")
        self.assertLess(local.index(mac), local.index(linux))
        self.assertLess(local.index(linux), next(i for i, s in enumerate(local) if s.get("name") == "Build artifacts"))
        self.assertIn("dist build", next(s for s in local if s.get("name") == "Build artifacts")["run"])

    def test_generation_drift_is_rejected(self):
        self.assertIn('Install dist (self-hosted macOS)', REPLACEMENT)
        self.assertIn('matrix.install_dist.run', ORIGINAL)
        with tempfile.TemporaryDirectory() as tmp:
            workflow = Path(tmp) / 'release.yml'
            expected = 'generated workflow with approved override'
            workflow.write_text(expected)
            check(workflow, expected)
            workflow.write_text(expected + '\n# manual edit')
            with self.assertRaisesRegex(SystemExit, 'differs from cargo-dist generation'):
                check(workflow, expected)

    def test_parallel_and_failed_installs(self):
        local = WORKFLOW["jobs"]["build-local-artifacts"]["steps"]
        mac = next(s for s in local if s.get("name") == "Install dist (self-hosted macOS)")
        # Replace only the GitHub expression: no network, no real Cargo home.
        script = mac["run"].replace("${{ matrix.install_dist.run }}", "curl --fake | sh")
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            fakebin = base / "fakebin"
            fakebin.mkdir()
            curl = fakebin / "curl"
            curl.write_text("#!/bin/sh\nif [ \"${FAIL_INSTALL:-0}\" = 1 ]; then exit 22; fi\n"
                            "if [ \"${IGNORE_INSTALL_DIR:-0}\" = 1 ]; then "
                            "printf '%s\\n' 'mkdir -p \"$HOME/.cargo/bin\"' "
                            "'touch \"$HOME/.cargo/bin/dist\"'; exit 0; fi\n"
                            "printf '%s\\n' 'mkdir -p \"$CARGO_DIST_INSTALL_DIR/bin\"' "
                            "'printf \"#!/bin/sh\\\\nexit 0\\\\n\" > \"$CARGO_DIST_INSTALL_DIR/bin/dist\"' "
                            "'chmod +x \"$CARGO_DIST_INSTALL_DIR/bin/dist\"'\n")
            curl.chmod(0o755)
            home = base / "home"
            home.mkdir()
            env = dict(os.environ, HOME=str(home), RUNNER_TEMP=str(base),
                       PATH=f"{fakebin}:{os.environ['PATH']}")
            outputs = [base / f"path-{i}" for i in range(4)]
            for output in outputs:
                output.touch()
            procs = [subprocess.Popen(["bash", "-c", script], env=dict(env, GITHUB_PATH=str(output),
                     FAIL_INSTALL="1" if i == 2 else "0", IGNORE_INSTALL_DIR="1" if i == 3 else "0"), stdout=subprocess.PIPE,
                     stderr=subprocess.PIPE, text=True) for i, output in enumerate(outputs)]
            results = [proc.communicate() for proc in procs]
            self.assertEqual([p.returncode for p in procs], [0, 0, 22, 1], results)
            roots = [output.read_text().strip() for output in outputs]
            self.assertEqual(roots[2:], ["", ""], "failed or unscoped installation must not publish a PATH")
            self.assertEqual(len(set(roots[:2])), 2, "concurrent jobs must use distinct roots")
            for root in roots[:2]:
                self.assertTrue(root.startswith(str(base) + "/project-canon-dist."))
                self.assertTrue((Path(root) / "dist").is_file())
                downstream = subprocess.run(['bash', '-c', 'command -v dist'],
                                            env=dict(env, PATH=f'{root}:{env["PATH"]}'),
                                            capture_output=True, text=True, check=True)
                self.assertEqual(downstream.stdout.strip(), str(Path(root) / 'dist'))
            # The wrong-directory mock deliberately writes here; the guard must
            # catch it and refuse to advertise a PATH for subsequent steps.
            self.assertTrue((home / '.cargo/bin/dist').exists())
            self.assertFalse((home / '.cargo/bin/dist').stat().st_mode & 0o111)
            missing_temp = subprocess.run(['bash', '-c', script], env=dict(env, RUNNER_TEMP='',
                                          GITHUB_PATH=str(base / 'missing-path')),
                                          capture_output=True, text=True)
            self.assertNotEqual(missing_temp.returncode, 0)
            self.assertFalse((base / 'missing-path').exists())


if __name__ == "__main__":
    unittest.main()
