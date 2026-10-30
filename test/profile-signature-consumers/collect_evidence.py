"""Export an allowlisted, credential-free E2E result before runtime cleanup.

Runtime gateway/SMTP/consumer logs deliberately stay private: they can contain
magic links and credentials. The public transcript is reconstructed from the
individual assertions, preserving their order and runner exit status.
"""
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def export(runtime: Path, output: Path, mode: str, run_id: str, exit_code: int, kind="e2e"):
    output.mkdir(parents=True, exist_ok=True)
    results = runtime / "results.jsonl"
    scenarios = [json.loads(line) for line in results.read_text().splitlines()] if results.exists() else []
    manifest = dict(schemaVersion=1, runId=run_id, mode=mode, runKind=kind,
                    endUtc=datetime.now(timezone.utc).isoformat(), exitCode=exit_code,
                    status="pass" if exit_code == 0 else "interrupted" if exit_code in (130, 143) else "fail", scenarios=scenarios,
                    rawLogs="private runtime only; public output includes assertions and exit status")
    prefix = "lifecycle" if kind == "benchmark" else "e2e"
    target = output / f"{prefix}-{mode}.json"
    target.write_text(json.dumps(manifest, indent=2) + "\n")
    transcript = "\n".join(f"PASS {item['id']}: {item['label']}" for item in scenarios)
    transcript += f"\nRUNNER EXIT {exit_code}\n"
    (output / f"{prefix}-{mode}.log").write_text(transcript)
    digest = hashlib.sha256(target.read_bytes()).hexdigest()
    (output / f"{prefix}-{mode}.sha256").write_text(f"{digest}  {target.name}\n")


if __name__ == "__main__":
    kind = "benchmark" if len(sys.argv) > 6 and sys.argv[6] == "1" else "e2e"
    export(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4], int(sys.argv[5]), kind)
