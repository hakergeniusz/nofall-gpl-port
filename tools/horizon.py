#!/usr/bin/env python3
"""Export pinned sources, apply the nofall bundle and build a native Horizon."""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
COMPAT = json.loads((ROOT / "compatibility.json").read_text())
PIN = COMPAT["horizon_revision"]
MIXINS = Path(os.environ.get("HORIZON_MIXINS", ROOT.parent / "horizon-mixins")).resolve()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def stage(horizon, stages):
    if not (horizon / ".git").exists():
        raise ValueError("--horizon must be a Git checkout (including a worktree)")
    subprocess.run(["git", "-C", str(horizon), "cat-file", "-e", PIN + "^{commit}"], check=True, capture_output=True)
    inputs = {"revision": PIN.encode(), "export-schema": b"2"}
    signature = digest(b"".join(k.encode() + b"\0" + value for k, value in sorted(inputs.items())))
    stages.mkdir(parents=True, exist_ok=True)
    out = stages / signature[:20]
    if out.exists():
        marker = json.loads((out / ".horizon-nofall.json").read_text())
        if marker["signature"] != signature:
            raise ValueError(f"stage marker mismatch: {out}")
        for path, expected in marker["files"].items():
            if digest((out / path).read_bytes()) != expected:
                raise ValueError(f"staged source was edited: {path}; remove this generated stage to rebuild it")
        return out, False
    temporary = Path(tempfile.mkdtemp(prefix=".mod-nofall-stage-", dir=stages))
    try:
        process = subprocess.Popen(["git", "-C", str(horizon), "archive", "--format=tar", PIN], stdout=subprocess.PIPE)
        try:
            with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                for member in archive:
                    archive.extract(member, temporary, filter="data")
        finally:
            process.stdout.close()
            code = process.wait()
        if code:
            raise ValueError(f"git archive exited {code}")
        for path, expected in COMPAT["source_sha256"].items():
            if digest((temporary / path).read_bytes()) != expected:
                raise ValueError(f"pinned source hash mismatch: {path}")
        # Materialize the committed documentation symlink for the mixin CLI's
        # symlink-free source snapshot. No Rust source is edited here.
        for file in temporary.rglob("*"):
            if file.is_symlink():
                content = file.read_bytes()
                file.unlink()
                file.write_bytes(content)
        (temporary / ".horizon-revision").write_text(PIN + "\n")
        # The source tree itself is deterministic. Cargo outputs live elsewhere.
        files = {str(p.relative_to(temporary)): digest(p.read_bytes())
                 for p in sorted(temporary.rglob("*")) if p.is_file()}
        (temporary / ".horizon-nofall.json").write_text(json.dumps(
            {"signature": signature, "horizon_revision": PIN, "files": files}, sort_keys=True, indent=2) + "\n")
        os.rename(temporary, out)
    except BaseException:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return out, True


def mixin_stage(horizon, stages):
    revision = subprocess.check_output(["git", "-C", str(MIXINS), "rev-parse", "HEAD"], text=True).strip()
    if revision != COMPAT["horizon_mixins_revision"]:
        raise ValueError(f"horizon-mixins must be pinned at {COMPAT['horizon_mixins_revision']}, got {revision}")
    subprocess.run(["git", "-C", str(MIXINS), "diff", "--quiet", "HEAD", "--", "Cargo.toml", "Cargo.lock", "crates"], check=True)
    original, _ = stage(horizon, stages / "original")
    subprocess.run(["cargo", "build", "--locked", "--offline", "--target-dir", str(MIXINS / "target"), "-p", "horizon-mixins-cli"], cwd=MIXINS, check=True, stdout=sys.stderr)
    cli = MIXINS / "target/debug/horizon-mixins"
    output = stages / "modded"
    plan = stages / "plan.json"
    base = ["--workspace", str(original), "--mods", str(ROOT / "mixins")]
    subprocess.run([str(cli), "plan", *base, "--out", str(plan)], check=True, capture_output=True, text=True)
    applied = subprocess.run([str(cli), "apply", *base, "--out", str(output)], check=True, capture_output=True, text=True)
    report = json.loads(applied.stdout)
    # The apply clears the mod API injections file the client's build.rs
    # reads (`horizon_mod_api_gen` takes what the env names, or nothing —
    # the launcher writes `[]` for a plain bundle). This bundle declares no
    # mod API hooks, so the file is an empty list again.
    (output / ".horizon-mod-api-injections.json").write_text("[]")
    return output, report["stage_changed"]


@contextlib.contextmanager
def lock(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def run_cargo(output, command, target, log_name):
    with lock(target.parent / (target.name + ".mod-nofall.lock")):
        run_start = time.monotonic()
        result = subprocess.run(command, cwd=output, capture_output=True, text=True)
    report = {"cargo_seconds": time.monotonic() - run_start, "cargo_command": command,
              "cargo_exit": result.returncode}
    log = ROOT / "target" / f"horizon-{log_name}.log"
    log.write_text(result.stdout + result.stderr)
    report["cargo_log"] = str(log)
    return report, result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["stage", "check", "build", "test"])
    parser.add_argument("--horizon", type=Path, default=ROOT.parent / "horizon")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target/horizon-cargo")
    args = parser.parse_args()
    horizon = args.horizon.resolve()
    target = args.target_dir.resolve()
    if target.is_relative_to(horizon):
        parser.error("Cargo target must be outside the original Horizon checkout")
    started = time.monotonic()
    with lock(ROOT / "target/stage.lock"):
        output, changed = mixin_stage(horizon, ROOT / "target/mixins")
        if target.is_relative_to(output) or output.is_relative_to(target):
            parser.error("Cargo target and generated source stage must be separate")
        report = {"stage": str(output), "stage_changed": changed,
                  "horizon_revision": PIN, "staging_seconds": time.monotonic()-started}
        if args.action != "stage":
            base = ["cargo", args.action, "--locked", "--offline",
                    "--no-default-features", "--target-dir", str(target)]
            if args.action == "build":
                command = [*base, "-p", "horizon-client", "--bin", "horizon-client", "--profile", "fast"]
            elif args.action == "check":
                # The whole game side: the game.rs module the runtime is, the
                # hooks the tick, list, settings and click helpers are
                # injected into, and landing's gate compiled as the lib
                # dependency both crates share.
                command = [*base, "-p", "horizon-client"]
            else:
                # The staged tests of both sides the mod touches, the filter
                # taking every test the mod names: first the landing's gate
                # (horizon-client-lib's unittests — the hurt the gate
                # cancels is a landing's), then the runtime as the game
                # module carries it (horizon-client's).
                command = [*base, "-p", "horizon-client-lib", "nofall", "--", "--test-threads", "4"]
            report_cargo, result = run_cargo(output, command, target, args.action if args.action != "test" else "test")
            report.update(cargo_seconds=report_cargo["cargo_seconds"], cargo_command=command,
                          cargo_exit=result.returncode, cargo_log=report_cargo["cargo_log"])
            if args.action == "test" and not result.returncode:
                command_client = [*base, "-p", "horizon-client", "nofall", "--", "--test-threads", "4"]
                report_client, result_client = run_cargo(output, command_client, target, "test-client")
                report.update(client_cargo_seconds=report_client["cargo_seconds"],
                              client_cargo_command=command_client,
                              client_cargo_exit=result_client.returncode,
                              client_cargo_log=report_client["cargo_log"])
                if result_client.returncode:
                    result = result_client
            if result.returncode:
                print(result.stderr[-12000:], file=sys.stderr)
                print(json.dumps(report), file=sys.stderr)
                return result.returncode
            if args.action == "build":
                report["binary"] = str(target / "fast/horizon-client")
        print(json.dumps(report, indent=2))
        return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        print(f"Native mod: {error}", file=sys.stderr)
        raise SystemExit(1)
