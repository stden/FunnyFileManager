#!/usr/bin/env python3
"""Purpose: Profile the optimized FFM binary on Windows using Visual Studio VSDiagnostics."""

from datetime import datetime, timezone
import os
from pathlib import Path
import shutil, subprocess, sys, tempfile, uuid

REPO = Path(__file__).resolve().parents[1]
COLLECTOR_REL = "Team Tools/DiagnosticsHub/Collector/VSDiagnostics.exe"
CPU_AGENT = "/loadAgent:4EA90761-2248-496C-B854-3C0399A591A4;DiagnosticsHub.CpuAgent.dll"


def default_runner(args, env=None, capture=True):
    cmd = [str(part) for part in args]
    if not capture:
        res = subprocess.run(cmd, env=env, check=False)
        return subprocess.CompletedProcess(cmd, res.returncode, "", "")
    return subprocess.run(
        cmd, env=env, capture_output=True, text=True,
        encoding="utf-8", errors="replace", check=False,
    )


def default_spawner(args, cwd):
    return subprocess.Popen([str(part) for part in args], cwd=str(cwd))


def _env_get(environ, name):
    upper = name.upper()
    return next((val for key, val in environ.items() if key.upper() == upper), None)


def _usable_collector(exe):
    col = Path(exe).parent
    return Path(exe).is_file() and (
        (col / "AgentConfigs/CpuUsageBase.json").is_file() or (col / "Agents/DiagnosticsHub.CpuAgent.dll").is_file()
    )


def _find_vswhere(environ, which):
    pf86 = _env_get(environ, "ProgramFiles(x86)") or ""
    fallback = Path(pf86) / "Microsoft Visual Studio/Installer/vswhere.exe"
    return which("vswhere") or (str(fallback) if pf86 and fallback.is_file() else None)


def find_vsdiagnostics(environ, which, runner):
    override = _env_get(environ, "FFM_VSDIAGNOSTICS")
    if override is not None:
        raw = override.strip()
        return (raw if Path(raw).is_file() else which(raw)) if raw else None
    return which("VSDiagnostics") or _vswhere_vsdiagnostics(environ, which, runner) or _glob_vsdiagnostics(environ)


def _vswhere_vsdiagnostics(environ, which, runner):
    vswhere = _find_vswhere(environ, which)
    if not vswhere:
        return None
    res = runner([vswhere, "-products", "*", "-property", "installationPath"])
    roots = res.stdout.splitlines() if res.returncode == 0 else []
    exes = [Path(line.strip()) / COLLECTOR_REL for line in roots if line.strip()]
    return next((str(exe) for exe in exes if _usable_collector(exe)), None)


def _glob_vsdiagnostics(environ):
    pattern = f"Microsoft Visual Studio/*/*/{COLLECTOR_REL}"
    for key in ("ProgramFiles", "ProgramFiles(x86)"):
        base = _env_get(environ, key) or ""
        matches = sorted(Path(base).glob(pattern), reverse=True) if base else []
        found = next((str(exe) for exe in matches if _usable_collector(exe)), None)
        if found:
            return found
    return None


def _check_tools(environ, which, runner):
    found = {}
    for name in ("cargo", "rustc", "git"):
        path = which(name)
        if not path:
            return None, f"Required command is missing: {name}\n"
        found[name] = path
    vsdiag = find_vsdiagnostics(environ, which, runner)
    if not vsdiag:
        return None, "Required command is missing: VSDiagnostics\n"
    found["vsdiagnostics"] = vsdiag
    return found, ""


def _build_release(repo, tools, environ, runner):
    host = runner([tools["rustc"], "--print", "host-tuple"])
    if host.returncode != 0:
        return "", host.returncode, host.stderr
    triple = host.stdout.strip()
    build_env = dict(environ)
    build_env["CARGO_PROFILE_RELEASE_DEBUG"] = "1"
    build_env["CARGO_PROFILE_RELEASE_STRIP"] = "none"
    cmd = [
        tools["cargo"], "build", "--release", "--bin", "ffm",
        "--target", triple, "--target-dir", str(repo / "target"),
    ]
    res = runner(cmd, env=build_env, capture=False)
    return triple, res.returncode, res.stderr or ""


def _write_metadata(capture, triple, tools, runner):
    rev = runner([tools["git"], "rev-parse", "HEAD"])
    if rev.returncode != 0:
        return rev.returncode, rev.stderr
    status = runner([tools["git"], "status", "--short"])
    rustc_v = runner([tools["rustc"], "--version", "--verbose"])
    cargo_v = runner([tools["cargo"], "--version"])
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    text = "".join([
        f"Captured at: {stamp}\n",
        "Build: release, default features, DEBUG=1, STRIP=none\n",
        f"Target: {triple}\nCPU sampling: VSDiagnostics (DiagnosticsHub.CpuAgent.dll)\n",
        f"Git revision: {rev.stdout}", status.stdout, rustc_v.stdout, cargo_v.stdout,
    ])
    (capture / "metadata.txt").write_text(text, encoding="utf-8")
    return 0, ""


def _prepare_capture(repo, triple, tools, runner):
    exe_name = "ffm.exe" if "windows" in triple else "ffm"
    release = repo / "target" / triple / "release"
    built_exe = release / exe_name
    if not built_exe.is_file():
        return None, None, 1, f"Built executable is missing: {built_exe}\n"
    profiles = repo / "target/profiles"
    profiles.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    capture = Path(tempfile.mkdtemp(prefix=f"ffm-{stamp}-", dir=str(profiles)))
    copied_exe = capture / exe_name
    shutil.copy2(built_exe, copied_exe)
    built_pdb = release / "ffm.pdb"
    if built_pdb.is_file():
        shutil.copy2(built_pdb, capture / "ffm.pdb")
    rc, err = _write_metadata(capture, triple, tools, runner)
    return (capture, copied_exe, 0, "") if rc == 0 else (capture, copied_exe, rc, err)


def _agent_flag(vsdiag):
    cfg = Path(vsdiag).resolve().parent / "AgentConfigs/CpuUsageBase.json"
    return f"/loadConfig:{cfg}" if cfg.is_file() else CPU_AGENT


def _collect_session(vsdiag, capture, exe_path, argv, repo, runner, spawner):
    sid = str(uuid.uuid4())
    start = runner([vsdiag, "start", sid, _agent_flag(vsdiag)])
    if start.returncode != 0:
        return start.returncode, start.stderr
    output = capture / "profile.diagsession"
    ffm_rc = 0
    try:
        child = spawner([exe_path, *argv], repo)
        if child.poll() is None:
            runner([vsdiag, "update", sid, f"/attach:{child.pid}"])
        ffm_rc = child.wait()
    finally:
        stop = runner([vsdiag, "stop", sid, f"/output:{output}"])
    if ffm_rc != 0:
        return ffm_rc, ""
    if stop.returncode != 0 or not output.is_file():
        return stop.returncode or 1, f"Missing {output.name}\n{stop.stderr}"
    return 0, ""


def run_profile(argv, *, repo=REPO, environ=None, which=shutil.which, runner=default_runner, spawner=default_spawner):
    env = dict(os.environ if environ is None else environ)
    tools, err = _check_tools(env, which, runner)
    if not tools:
        return 1, err
    triple, rc, err = _build_release(repo, tools, env, runner)
    if rc != 0:
        return rc, err
    capture, exe_path, rc, err = _prepare_capture(repo, triple, tools, runner)
    if rc != 0:
        return rc, err
    print(f"FFM CPU profile: {capture}\nUse FFM normally, then exit with F10 to save profile.diagsession.")
    print("Action timings are also saved in your FFM configuration directory (performance/).")
    return _collect_session(tools["vsdiagnostics"], capture, exe_path, argv, repo, runner, spawner)


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    os.chdir(REPO)
    rc, err = run_profile(args)
    if err:
        sys.stderr.write(err)
    return rc


if __name__ == "__main__":
    sys.exit(main())
