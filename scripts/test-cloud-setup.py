#!/usr/bin/env python3
"""Exercise cloud bootstrap control flow with isolated command doubles.

No package installation, networking, or writes outside a TemporaryDirectory.
The doubles report commands rather than proving remote services are available.

PATH holds only the doubles and a directory of plain utilities linked from the
host (HOST_UTILITIES), and setup's bin directory is redirected into the
temporary tree. A host tool such as a runner's real npm or dotnet can therefore
never stand in for a double: a command without one fails as not found.
"""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

FAKE = r"""#!/usr/bin/python3
import hashlib, io, json, os, pathlib, subprocess, sys, tarfile
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['SETUP_LOG'], 'a') as log:
    log.write(json.dumps([name, *args]) + '\n')
if name == os.environ.get('FAIL_TOOL'):
    sys.exit(42)
if name == 'id':
    print(os.environ.get('FAKE_UID', '0'))
elif name == 'sudo':
    if os.environ.get('NO_SUDO'):
        sys.exit(1)
    sys.exit(subprocess.call(args[1:]))
elif name == 'python3':
    if args[:2] == ['-m', 'pip']:
        pass
    else:
        sys.exit(subprocess.call([os.environ['REAL_PYTHON'], *args]))
elif name == 'node':
    if args == ['--version']:
        print('v22.23.3')
    elif args and args[0].endswith('playwright-core/cli.js'):
        if os.environ.get('FAIL_PLAYWRIGHT'):
            sys.exit(1)
    elif os.environ.get('OLD_NODE'):
        sys.exit(1)
elif name == 'dotnet':
    if args == ['--list-sdks']:
        print(os.environ.get('DOTNET_SDK', '8.0.414') + ' [/usr/share/dotnet/sdk]')
elif name == 'apt-cache':
    if os.environ.get('NO_DOTNET_PACKAGE'):
        sys.exit(100)
elif name == 'uv':
    if os.environ.get('FAIL_UV'):
        sys.exit(2)
elif name == 'wasm-bindgen':
    print('wasm-bindgen ' + os.environ['WASM_VERSION'])
elif name == 'pkl':
    if args == ['--version']:
        print('Pkl ' + os.environ.get('PKL_VERSION', '0.32.1') + ' (Linux)')
elif name == 'git' and 'rev-parse' in args:
    if os.environ.get('WRONG_CORPUS'):
        print('mismatched-existing-revision')
        sys.exit(0)
    print('2c552eae435d575f1df7c11ff9ea19e3d2a324ce' if 'foundation-API' in str(args) else '938bd7636e094c2dc3c65ad6dae846ac3c7b6a9b')
elif name == 'mkdir':
    # Only the system bin directory is intercepted; scratch dirs are real.
    if '/usr/local/bin' not in args:
        sys.exit(subprocess.call(['/bin/mkdir', *args]))
elif name == 'curl':
    output = pathlib.Path(args[args.index('--output') + 1])
    if not os.environ.get('ALLOW_DOWNLOAD'):
        raise SystemExit('unexpected download in the preinstalled-tools scenario')
    archive = output.parent / 'node-v22.23.3-linux-x64.tar.xz'
    if output.name.endswith('.tar.xz'):
        with tarfile.open(output, 'w:xz') as tar:
            for tool in ['node', 'npm', 'npx']:
                data = pathlib.Path(os.environ['FAKE_BIN'], tool).read_bytes()
                info = tarfile.TarInfo('node/bin/' + tool)
                info.mode = 0o755
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
    elif output.name == 'SHASUMS256.txt':
        digest = '0' * 64 if os.environ.get('BAD_CHECKSUM') else hashlib.sha256(archive.read_bytes()).hexdigest()
        output.write_text(digest + '  ' + archive.name + '\n')
    elif output.name == 'rustup-init.sh':
        output.write_text('#!/usr/bin/env bash\ncp "$FAKE_BIN/cargo" "$CARGO_HOME/bin/rustup"\nchmod +x "$CARGO_HOME/bin/rustup"\n')
    else:
        output.write_text('deliberately invalid download')
"""


# Plain utilities the script and the doubles' shell shims need from the host.
HOST_UTILITIES = [
    "bash", "cat", "chmod", "cp", "dirname", "env", "grep", "mktemp", "rm",
    "sha256sum", "sort", "tar", "tr", "true", "uname", "xz",
]


class CloudSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="cloud-setup-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repository with spaces"
        self.bin = self.root / "bin"
        self.bin.mkdir()
        source = Path(__file__).resolve().parent.parent
        self.repo.mkdir()
        # Only the metadata needed by setup, never the whole source/build tree.
        for name in [
            "Cargo.lock",
            "rust-toolchain.toml",
            "package.json",
            "package-lock.json",
            ".config/dotnet-tools.json",
            "crates/openbim-ifc-wasm/tools/package.json",
            "crates/openbim-ifc-wasm/tools/package-lock.json",
            "crates/openbim-ifc-py/scripts/check-python.sh",
        ]:
            if (source / name).exists():
                (self.repo / name).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(source / name, self.repo / name)
        (self.repo / "scripts").mkdir()
        shutil.copy(source / "scripts/cloud-setup.sh", self.repo / "scripts")
        for script in ["fetch-ifc-schemas.sh", "fetch-official-references.py"]:
            path = self.repo / "scripts" / script
            path.write_text('#!/usr/bin/env bash\nprintf "reference fetch\\n"\n')
            path.chmod(0o755)
        (self.repo / "packages/example").mkdir(parents=True)
        (self.repo / "packages/example/PklProject").touch()
        (self.repo / "examples").mkdir()
        self.log = self.root / "commands.jsonl"
        home = self.root / "home"
        cargo_bin = home / ".cargo/bin"
        cargo_bin.mkdir(parents=True)
        tools = [
            "id",
            "sudo",
            "apt-get",
            "python3",
            "cargo",
            "rustup",
            "node",
            "npm",
            "npx",
            "dotnet",
            "apt-cache",
            "wasm-bindgen",
            "uv",
            "maturin",
            "mkdocs",
            "pkl",
            "git",
            "ln",
            "mkdir",
            "install",
            "chromium",
            "curl",
        ]
        for tool in tools:
            path = self.bin / tool
            path.write_text(FAKE)
            path.chmod(0o755)
        host = self.root / "host-bin"
        host.mkdir()
        for tool in HOST_UTILITIES:
            found = shutil.which(tool)
            if found is None:
                self.skipTest(f"host utility {tool} not found")
            (host / tool).symlink_to(found)
        (self.root / "system-bin").mkdir()
        for tool in [
            "rustup",
            "cargo",
            "rustc",
            "rustfmt",
            "cargo-fmt",
            "cargo-clippy",
            "clippy-driver",
            "wasm-bindgen",
            "cargo-deny",
        ]:
            (cargo_bin / tool).symlink_to(self.bin / (tool if tool in tools else "cargo"))
        import re

        lock = (source / "Cargo.lock").read_text() if (source / "Cargo.lock").exists() else ""
        match = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', lock)
        self.env = {
            **os.environ,
            "PATH": f"{self.bin}:{host}",
            "OPENBIM_CLOUD_BIN_DIR": str(self.root / "system-bin"),
            "HOME": str(home),
            "CARGO_HOME": str(home / ".cargo"),
            "SETUP_LOG": str(self.log),
            "REAL_PYTHON": shutil.which("python3"),
            "FAKE_BIN": str(self.bin),
            "WASM_VERSION": match[1] if match else "unused",
            "OPENCDE_REFERENCE_ROOT": str(self.root / "corpus"),
        }
        for name in [
            "FAIL_TOOL",
            "NO_SUDO",
            "FAKE_UID",
            "CHROME_BIN",
            "OLD_NODE",
            "ALLOW_DOWNLOAD",
            "BAD_CHECKSUM",
            "PKL_VERSION",
            "WRONG_CORPUS",
            "FAIL_PLAYWRIGHT",
            "DOTNET_SDK",
            "NO_DOTNET_PACKAGE",
            "FAIL_UV",
        ]:
            self.env.pop(name, None)
        (self.root / "corpus").mkdir()
        for name in ["foundation-API", "documents-API"]:
            (self.root / "corpus" / name).mkdir()

    def run_setup(self, *args, **env):
        return subprocess.run(
            ["/bin/bash", str(self.repo / "scripts/cloud-setup.sh"), *args],
            cwd=self.root,
            env={**self.env, **env},
            text=True,
            capture_output=True,
            check=False,
        )

    def commands(self):
        return (
            [json.loads(line) for line in self.log.read_text().splitlines()]
            if self.log.exists()
            else []
        )

    def test_help_has_no_side_effects(self):
        result = self.run_setup("--help")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.commands(), [])

    def test_unknown_option_has_no_side_effects(self):
        result = self.run_setup("--unknown")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.commands(), [])

    def test_root_setup_and_rerun_from_another_directory(self):
        for _ in range(2):
            result = self.run_setup()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("Cloud setup complete", result.stdout)
        commands = self.commands()
        self.assertTrue(any(c[:2] == ["apt-get", "install"] for c in commands))
        if (self.repo / "Cargo.lock").exists():
            self.assertIn(["cargo", "fetch", "--locked"], commands)
        self.assertFalse(any("install" in c and c[0] == "cargo" for c in commands))
        # Both lockfiles install through the npm double, never the host's npm.
        self.assertIn(["npm", "ci", "--no-audit", "--no-fund"], commands)
        self.assertIn(
            ["npm", "--prefix", "crates/openbim-ifc-wasm/tools", "ci", "--no-audit", "--no-fund"],
            commands,
        )
        self.assertNotIn("Optional extras skipped", result.stdout)

    def test_nonroot_uses_noninteractive_sudo(self):
        result = self.run_setup(FAKE_UID="1000")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(any(c[:2] == ["sudo", "-n"] for c in self.commands()))

    def test_missing_privileges_fail_before_install(self):
        result = self.run_setup(FAKE_UID="1000", NO_SUDO="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("passwordless sudo", result.stderr)
        self.assertFalse(any(c[0] == "apt-get" for c in self.commands()))

    def test_package_failure_stops_setup(self):
        result = self.run_setup(FAIL_TOOL="apt-get")
        self.assertEqual(result.returncode, 42)
        self.assertNotIn("Cloud setup complete", result.stdout)
        self.assertFalse(any(c[0] == "cargo" for c in self.commands()))

    def test_node_download_checks_integrity(self):
        if "node_version=" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not need Node")
        result = self.run_setup(OLD_NODE="1", ALLOW_DOWNLOAD="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(any(c[0] == "curl" for c in self.commands()))

    def test_bad_node_checksum_stops_before_exposure(self):
        if "node_version=" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not need Node")
        result = self.run_setup(OLD_NODE="1", ALLOW_DOWNLOAD="1", BAD_CHECKSUM="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Cloud setup complete", result.stdout)
        self.assertFalse(
            any(c[0] == "ln" and c[-1] == "/usr/local/bin/node" for c in self.commands())
        )

    def test_wasm_mismatch_installs_lockfile_version(self):
        if "wasm_version=" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not need wasm-bindgen")
        expected = self.env["WASM_VERSION"]
        result = self.run_setup(WASM_VERSION="0.0.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            [
                "cargo",
                "+stable",
                "install",
                "wasm-bindgen-cli",
                "--version",
                expected,
                "--locked",
                "--force",
            ],
            self.commands(),
        )

    def test_missing_browser_installs_user_cache(self):
        if "browser_found=" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not need Chromium")
        (self.bin / "chromium").unlink()
        result = self.run_setup(FAKE_UID="1000")
        self.assertEqual(result.returncode, 0, result.stderr)
        cli = str(self.repo / "node_modules/playwright-core/cli.js")
        # System libraries as root, the browser into the user's own cache.
        self.assertIn(["sudo", "-n", str(self.bin / "node"), cli, "install-deps", "chromium"],
                      self.commands())
        self.assertIn(["node", cli, "install", "chromium"], self.commands())

    def test_browser_failure_is_optional(self):
        (self.bin / "chromium").unlink()
        result = self.run_setup(FAIL_PLAYWRIGHT="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("skipped Chromium", result.stderr)
        self.assertIn("IFC_SKIP_BROWSER=1", result.stdout)
        self.assertIn("Cloud setup complete", result.stdout)

    def test_cli_targets_and_deb_tools(self):
        result = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = self.commands()
        install = next(c for c in commands if c[:2] == ["apt-get", "install"])
        self.assertIn("dpkg", install)  # dpkg-deb, for check-deb.sh
        targets = next(c for c in commands if c[:3] == ["rustup", "target", "add"])
        self.assertIn("wasm32-unknown-unknown", targets)
        self.assertTrue(any(t.endswith("-unknown-linux-musl") for t in targets))

    def test_dotnet_sdk_restores_docfx(self):
        result = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(["dotnet", "tool", "restore"], self.commands())
        self.assertFalse(any("dotnet-sdk-8.0" in c for c in self.commands()))

    def test_missing_dotnet_installs_ubuntu_package(self):
        (self.bin / "dotnet").unlink()
        result = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(any(c[0] == "apt-get" and "dotnet-sdk-8.0" in c for c in self.commands()))
        # The double installs nothing, so setup reports the skip and goes on.
        self.assertIn("skipped .NET 8 SDK", result.stderr)
        self.assertIn("Cloud setup complete", result.stdout)

    def test_dotnet_without_package_is_optional(self):
        result = self.run_setup(DOTNET_SDK="6.0.100", NO_DOTNET_PACKAGE="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(any(c[0] == "apt-get" and "dotnet-sdk-8.0" in c for c in self.commands()))
        self.assertNotIn(["dotnet", "tool", "restore"], self.commands())
        self.assertIn("skipped .NET 8 SDK", result.stderr)

    def test_python_tool_cache_uses_gate_pins(self):
        script = self.repo / "crates/openbim-ifc-py/scripts/check-python.sh"
        if not script.exists():
            self.skipTest("Workspace has no Python binding")
        result = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        install = next(c for c in self.commands() if c[:3] == ["uv", "pip", "install"])
        self.assertTrue(any(p.startswith("pdoc==") for p in install), install)
        failed = self.run_setup(FAIL_UV="1")
        self.assertEqual(failed.returncode, 0, failed.stderr)
        self.assertIn("skipped Python tool cache", failed.stderr)

    def test_bad_pkl_checksum_stops_setup(self):
        if "pkl-linux-amd64" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not need Pkl")
        result = self.run_setup(PKL_VERSION="0.0.0", ALLOW_DOWNLOAD="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Cloud setup complete", result.stdout)
        self.assertFalse(any(c[0] == "install" for c in self.commands()))

    def test_rustup_bootstrap_when_missing(self):
        if not (self.repo / "Cargo.lock").exists():
            self.skipTest("Pkl does not use Rust")
        (self.bin / "rustup").unlink()
        (Path(self.env["CARGO_HOME"]) / "bin/rustup").unlink()
        result = self.run_setup(ALLOW_DOWNLOAD="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(
            any(c[0] == "curl" and "https://sh.rustup.rs" in c for c in self.commands())
        )
        self.assertTrue(any(c[:3] == ["rustup", "toolchain", "install"] for c in self.commands()))

    def test_existing_cde_corpus_is_preserved_on_mismatch(self):
        if "OPENCDE_REFERENCE_ROOT" not in (self.repo / "scripts/cloud-setup.sh").read_text():
            self.skipTest("Workspace does not use the OpenCDE corpus")
        result = self.run_setup(WRONG_CORPUS="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertFalse(any(c[0] == "git" and "checkout" in c for c in self.commands()))

    def test_dependency_failure_stops_setup(self):
        if not (self.repo / "Cargo.lock").exists():
            self.skipTest("Pkl has no Cargo dependencies")
        result = self.run_setup(FAIL_TOOL="cargo")
        self.assertEqual(result.returncode, 42)
        self.assertNotIn("Cloud setup complete", result.stdout)


if __name__ == "__main__":
    unittest.main()
