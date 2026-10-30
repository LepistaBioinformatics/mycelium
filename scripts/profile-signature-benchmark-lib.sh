#!/usr/bin/env bash
# Run only after correctness passes and release binaries have been built.
benchmark_config() {
    curl -fsS -H "Authorization: Bearer $JWT" -H "x-psig-control: $PSIG_CONTROL_TOKEN" \
        "http://127.0.0.1:$API_PORT/express-svc/capture" >"$WORKDIR/benchmark-pair.json"
    JWT="$JWT" "$PYTHON_BIN" - "$WORKDIR" "$MODE" "$1" "$2" "$API_PORT" "$API_PID" "$EXPRESS_PID" "$BENCHMARK_OUTPUT" <<'PY'
import hashlib, json, os, sys
from pathlib import Path
from myc_http_tools.functions.decode_and_decompress_profile_from_base64 import decode_and_decompress_profile_from_base64_robust
runtime, mode, state, dataset, port, gateway, consumer, output = sys.argv[1:]
pair = json.loads((Path(runtime)/'benchmark-pair.json').read_text())
assert pair['profile']
profile = decode_and_decompress_profile_from_base64_robust(pair['profile'])
profile_json_bytes = len(profile.model_dump_json(by_alias=True).encode())
assert profile_json_bytes >= 30000 if dataset == 'large' else profile_json_bytes < 4000
value = dict(mode=mode, framework='express', state=state, dataset=dataset,
 url=f'http://127.0.0.1:{port}/express-svc/whoami',
 headers={'Authorization': 'Bearer '+os.environ['JWT']}, account=os.environ['ACC_ID'],
 profileBytes=len(pair['profile']), profileJsonBytes=profile_json_bytes, signatureBytes=len(pair['signature'] or ''),
 datasetSha256=hashlib.sha256(pair['profile'].encode()).hexdigest(),
 pids=[int(gateway), int(consumer)], cpuTicksPerSecond=os.sysconf('SC_CLK_TCK'), output=output)
(Path(runtime)/'benchmark-config.json').write_text(json.dumps(value))
PY
}

run_benchmark_matrix() {
    export ACC_ID
    BENCHMARK_OUTPUT="${EVIDENCE_DIR:?--evidence-dir required}/benchmark-$MODE.jsonl"
    mkdir -p "$EVIDENCE_DIR"
    : >"$BENCHMARK_OUTPUT"
    for dataset in small large; do
        if [ "$dataset" = large ]; then
            stop_pid "$API_PID"; API_PID=""
            if [ "$MODE" = standalone ]; then
                "$PYTHON_BIN" "$CONSUMERS/benchmark_fixture.py" "$WORKDIR/data/mycelium.db" "$ACC_ID" "$EMAIL"
            else
                "$PYTHON_BIN" "$CONSUMERS/benchmark_fixture.py" sql "$ACC_ID" "$EMAIL" | \
                    compose exec -T postgres psql -U postgres -d psig -v ON_ERROR_STOP=1 >"$WORKDIR/bench-fixture.log" 2>&1
            fi
        fi
        for repetition in ${PSIG_BENCH_REPETITIONS:-1 2 3}; do
            states='A B C'
            [ "$repetition" != 2 ] || states='C A B'
            [ "$repetition" != 3 ] || states='B C A'
            [ -z "${PSIG_BENCH_STATES:-}" ] || states="$PSIG_BENCH_STATES"
            for state in $states; do
                stop_pid "$API_PID"; API_PID=""
                stop_consumers
                if [ "$MODE" = full ]; then compose exec -T redis redis-cli --no-auth-warning -a "$PSIG_REDIS_PASSWORD" FLUSHDB >"$WORKDIR/flush.log" 2>&1; fi
                if [ "$MODE" = postgres-only ]; then
                    compose exec -T postgres psql -U postgres -d psig -v ON_ERROR_STOP=1 \
                        -c 'DELETE FROM kv_artifact;' >"$WORKDIR/flush.log" 2>&1
                fi
                if [ "$state" = C ]; then start_consumers '1'; else start_consumers ''; fi
                if [ "$state" = A ]; then write_config ''; else write_config "$(signing_block 1 '1')"; fi
                start_gateway
                benchmark_config "$state" "$dataset"
                for concurrency in ${PSIG_BENCH_CONCURRENCIES:-1 8 32}; do
                    "$NODE_BIN" "$REPO_ROOT/scripts/profile-signature-benchmark.mjs" "$WORKDIR/benchmark-config.json" "$concurrency" "$repetition"
                done
                if [ "$state" = C ] && [ "$repetition" = 1 ]; then
                    "$PYTHON_BIN" - "$WORKDIR" "$EXPRESS_PORT" <<'PY'
import json, sys
from pathlib import Path
runtime, port = Path(sys.argv[1]), sys.argv[2]
config = json.loads((runtime/'benchmark-config.json').read_text())
pair = json.loads((runtime/'benchmark-pair.json').read_text())
config.update(url=f'http://127.0.0.1:{port}/whoami', pathKind='direct',
 headers={'x-mycelium-profile':pair['profile'], 'x-mycelium-profile-signature':pair['signature']})
(runtime/'benchmark-direct.json').write_text(json.dumps(config))
PY
                    for concurrency in ${PSIG_BENCH_CONCURRENCIES:-1 8 32}; do
                        "$NODE_BIN" "$REPO_ROOT/scripts/profile-signature-benchmark.mjs" "$WORKDIR/benchmark-direct.json" "$concurrency" "$repetition"
                    done
                fi
            done
        done
    done
}
