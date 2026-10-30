"""Purpose: Isolate profiling launcher tests with recording compiler and profiler fakes."""

from sandbox import SandboxTestCase


NATIVE_TARGET = "x86_64-unknown-linux-gnu"


class ProfilingCase(SandboxTestCase):
    def setUp(self):
        super().setUp()
        sandbox = self.sandbox
        for script in ("x.sh", "sx.sh"):
            sandbox.copy(script)
        self.template = sandbox.fake(
            "profiled-ffm", path=sandbox.root / "built-ffm",
            effect='sys.exit(int(os.environ.get("FFM_TEST_STATUS", "0")))',
        )
        self.binary = sandbox.repo / "target" / NATIVE_TARGET / "release/ffm"
        sandbox.fake("git", effect='''
            print("revision-for-this-capture" if args == ["rev-parse", "HEAD"]
                  else " M src/example.rs")
        ''')
        sandbox.fake("rustc", effect=f'''
            print({NATIVE_TARGET!r} if args == ["--print", "host-tuple"]
                  else "rustc test-compiler")
        ''')
        sandbox.fake("cargo", effect=f'''
            from pathlib import Path
            import shutil
            if args == ["--version"]:
                print("cargo test-builder")
                sys.exit(0)
            target = Path(args[args.index("--target-dir") + 1]) if "--target-dir" in args else Path("target")
            triple = args[args.index("--target") + 1] if "--target" in args else os.environ.get("CARGO_BUILD_TARGET")
            if triple:
                target = target / triple
            binary = target / "release/ffm"
            binary.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2({str(self.template)!r}, binary)
        ''')
        sandbox.fake("flamegraph", effect='''
            from pathlib import Path
            import subprocess
            record = args[args.index("--cmd") + 1].split()
            command = args[args.index("--") + 1:]
            result = subprocess.run(["perf", *record, "--", *command], check=False)
            if result.returncode:
                sys.exit(1)
            Path(args[args.index("--output") + 1]).write_text("captured SVG")
        ''')
        sandbox.fake("perf", effect='''
            from pathlib import Path
            import subprocess
            Path(args[args.index("-o") + 1]).write_text("raw CPU samples")
            result = subprocess.run(args[args.index("--") + 1:], check=False)
            sys.exit(result.returncode)
        ''')

    def launch(self, *args, env=None):
        return self.sandbox.execute(
            [self.sandbox.repo / "x.sh", *args], cwd=self.sandbox.home, env=env,
        )

    def captures(self):
        return sorted((self.sandbox.repo / "target/profiles").glob("ffm-*"))

