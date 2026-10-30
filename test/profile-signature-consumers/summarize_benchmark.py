"""Validate every planned benchmark cell and preserve raw samples as lossless gzip."""
import gzip
import hashlib
import itertools
import json
import statistics
import sys
from pathlib import Path


def summarize(directory):
    rows, artifacts = [], []
    expected = set(itertools.product(("A", "B", "C"), ("small", "large"), (1, 8, 32), (1, 2, 3)))
    for mode in ("standalone", "postgres-only", "full"):
        path = directory / f"benchmark-{mode}.jsonl"
        raw = path.read_bytes()
        records = [json.loads(line) for line in raw.splitlines()]
        actual = {(r["state"], r["dataset"], r["concurrency"], r["repetition"]) for r in records if r["pathKind"] == "gateway"}
        assert actual == expected, f"{mode}: incomplete matrix"
        assert len([r for r in records if r["pathKind"] == "gateway"]) == len(expected)
        for r in records:
            assert r["errors"] == 0 and r["count"] >= 200 and len(r["samples"]) == r["count"]
            rows.append({k: value for k, value in r.items() if k != "samples"})
        compressed = gzip.compress(raw, mtime=0)
        assert gzip.decompress(compressed) == raw
        archive = path.with_suffix(path.suffix + ".gz")
        archive.write_bytes(compressed)
        artifacts.append(dict(mode=mode, file=archive.name, sha256=hashlib.sha256(compressed).hexdigest(),
                              uncompressedSha256=hashlib.sha256(raw).hexdigest(), records=len(records)))
        # Retain the original until all modes have passed validation.
    comparisons = []
    for mode, dataset, concurrency in itertools.product(("standalone", "postgres-only", "full"), ("small", "large"), (1, 8, 32)):
        selection = [r for r in rows if (r["mode"], r["dataset"], r["concurrency"], r["pathKind"]) == (mode, dataset, concurrency, "gateway")]
        states = {state: [r for r in selection if r["state"] == state] for state in ("A", "B", "C")}
        comparisons.append(dict(mode=mode, dataset=dataset, concurrency=concurrency,
            states={state: dict(p50Ms=statistics.median(r["p50Ms"] for r in group),
                                p95Ms=statistics.median(r["p95Ms"] for r in group),
                                p99Ms=statistics.median(r["p99Ms"] for r in group),
                                throughputPerSecond=statistics.median(r["throughputPerSecond"] for r in group))
                    for state, group in states.items()},
            signingDeltaP50Ms=statistics.median(states["B"][i]["p50Ms"]-states["A"][i]["p50Ms"] for i in range(3)),
            verificationDeltaP50Ms=statistics.median(states["C"][i]["p50Ms"]-states["B"][i]["p50Ms"] for i in range(3))))
    result = dict(schemaVersion=1, framework="express", gatewayCells=162,
                  totalRequests=sum(r["count"] for r in rows), errors=sum(r["errors"] for r in rows),
                  artifacts=artifacts, comparisons=comparisons, measurements=rows,
                  method="Median of three per-run nearest-rank percentiles; deltas paired by repetition. Direct floor kept separate. No SLA.")
    (directory / "benchmark.json").write_text(json.dumps(result, indent=2) + "\n")
    for mode in ("standalone", "postgres-only", "full"):
        (directory / f"benchmark-{mode}.jsonl").unlink()
    print(f"PASS 162 gateway cells + direct floor; {result['totalRequests']} requests, zero unexpected errors")


if __name__ == "__main__":
    summarize(Path(sys.argv[1]))
