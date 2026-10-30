"""Record a bounded local validation command with its actual result and log hash.

Pass only credential-free commands. E2E credentials must remain environment/file
inputs; its public transcript contains assertions only.
"""
import argparse
import hashlib
import json
import os
import signal
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path


def run(args):
    args.evidence.mkdir(parents=True, exist_ok=True)
    attempt = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%f")
    log = args.evidence / "logs" / (args.id + "-" + attempt + ".log")
    log.parent.mkdir(exist_ok=True)
    start = datetime.now(timezone.utc).isoformat()
    timer = time.monotonic()
    timed_out = False
    with log.open("w") as output:
        child = subprocess.Popen(args.command, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = child.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            code = 124
    step = dict(id=args.id, attempt=attempt, command=args.command, cwd=str(Path.cwd()), startUtc=start,
                endUtc=datetime.now(timezone.utc).isoformat(), durationSeconds=time.monotonic()-timer,
                exitCode=code, status="pass" if code == 0 else "interrupted" if code in (130, 143, -2, -15) else "fail",
                failureCategory="timeout" if timed_out else "interrupted" if code in (130, 143, -2, -15) else None,
                logPath=str(log.relative_to(args.evidence)), logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    with (args.evidence / "commands.jsonl").open("a") as stream:
        stream.write(json.dumps(step) + "\n")
    print(f"{step['status'].upper()} {args.id}: exit={code}, {step['durationSeconds']:.2f}s", flush=True)
    return code


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--id", required=True)
    parser.add_argument("--timeout", type=int, default=1800)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command or not args.id.replace("-", "").replace("_", "").isalnum():
        parser.error("command and safe step id required")
    raise SystemExit(run(args))
