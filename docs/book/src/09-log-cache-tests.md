# Log Tests for Cache and Identity Flow

This guide explains how to run log-based validation for the security group, identity, and profile resolution flow—including cache behaviour (JWKS, email, and profile caches). The validation script checks that the expected sequence of stages appears in the logs and helps you verify cache hits and misses.

## Overview

The API emits structured log events for:

- **identity.jwks**: Resolution of JSON Web Key Sets (from cache or via URI)
- **identity.email**: Resolution of user email (from cache, from token, or via userinfo)
- **identity.profile**: Resolution of user profile (from cache or via datastore)
- **router.***: Security group check and identity/profile injection

Each event includes a `stage` and, when applicable, an `outcome` (e.g. `from_cache`, `resolved`, `from_token`) and `cache_hit` (true/false). The validation script parses these events and checks that every “start” stage is followed by a matching “outcome” event.

## Prerequisites

- **Python 3** (3.9 or higher recommended)
- Log output from a running Mycelium API (tracing format with timestamps and levels)

No extra Python packages are required; the script uses only the standard library.

## Step 1: Save Logs to a File

To validate logs, you need to capture the API’s stdout (and optionally stderr) into a file while the API is running.

### Option A: Redirect stdout when starting the API

If you start the API from a shell, redirect stdout to a file:

```bash
# Run the API and append all stdout to a log file
cargo run -p mycelium-api --bin myc_api -- --config settings/config.dev.for-docker.toml >> api.log 2>&1
```

To overwrite the file instead of appending, use a single `>`:

```bash
cargo run -p mycelium-api --bin myc_api -- --config settings/config.dev.for-docker.toml > api.log 2>&1
```

### Option B: Redirect stdout of an already running process

If the API is already running (e.g. in Docker or another terminal), you can capture logs by attaching to the process or by configuring your runtime to write logs to a file. For example, with Docker:

```bash
docker compose logs -f mycelium-api > api.log 2>&1
```

Stop capturing when you have enough requests (e.g. after a few authenticated/protected calls).

### Option C: Use a log level that includes INFO

The validation script relies on **INFO**-level events for `stage` and `outcome`. Ensure the API is not running with a log level that filters them out (e.g. `RUST_LOG=info` or `RUST_LOG=myc_api=info`). Default or `info` level is usually sufficient.

Example with explicit log level:

```bash
RUST_LOG=info cargo run -p mycelium-api --bin myc_api -- --config settings/config.dev.for-docker.toml >> api.log 2>&1
```

After reproducing the flows you care about (authenticated and/or protected requests), stop the capture. The resulting file (e.g. `api.log`) is the input for the validation script.

## Step 2: Run the Validation Script

The script lives in the repository at `scripts/python/evaluate_security_group_logs.py`.

### Basic usage

Pass the log file path as the first argument:

```bash
python3 scripts/python/evaluate_security_group_logs.py api.log
```

Or using an absolute path:

```bash
python3 /path/to/mycelium/scripts/python/evaluate_security_group_logs.py /path/to/api.log
```

### Verbose output (recommended for interpretation)

To see the full sequence of stages and how they are grouped into **cycles**, use `--verbose` or `-v`:

```bash
python3 scripts/python/evaluate_security_group_logs.py api.log --verbose
```

Verbose mode prints:

- The total number of stage events found
- For each **cycle**, a header like `--- Cycle N (M events) ---` and the list of events in that cycle (timestamp, stage, outcome, cache_hit)
- A blank line between cycles for readability

### Exit codes

- **0**: All sequences validated successfully (OK).
- **1**: One or more sequence violations (e.g. a stage started but no matching outcome found).
- **2**: Usage error (e.g. missing log file path).

You can use the exit code in scripts or CI:

```bash
python3 scripts/python/evaluate_security_group_logs.py api.log
if [ $? -eq 0 ]; then
  echo "Log validation passed"
else
  echo "Log validation failed"
  exit 1
fi
```

## Step 3: How to Interpret the Output

### Summary block

At the top, the script prints:

- **Total stage events found**: Number of log lines that contained a `stage=` field. This is the number of events used for validation and (in verbose mode) for the cycle listing.

### Result

- **Result: OK** – Every “start” stage (e.g. `identity.jwks`, `identity.external`, `identity.profile`) had a matching “outcome” event in the expected order. No violations were reported.
- **Result: FAIL** – The script lists violations. Each violation message includes the timestamp and a short description (e.g. “identity.jwks started but no identity.jwks outcome (from_cache/resolved) found”). Fix by ensuring the API actually completes that step and emits the corresponding log.

### Verbose output: cycles

When you use `--verbose`, events are grouped into **cycles**. A new cycle starts at each `identity.external` (start of identity resolution for a request). So:

- **One cycle** corresponds to one logical “identity + profile resolution” flow (e.g. one authenticated or protected request).
- Within a cycle you see the order of stages, for example:
  - `identity.external` (start)
  - `identity.jwks` → then `identity.jwks outcome=from_cache` or `outcome=resolved`
  - `identity.email` → then optional `identity.email.cache cache_hit=true/false` → then `identity.email outcome=from_cache` or `outcome=resolved`
  - `identity.external outcome=ok`
  - `identity.profile` → optional `identity.profile.cache cache_hit=true/false` → `identity.profile outcome=from_cache` or `outcome=resolved`

### Interpreting cache behaviour

- **JWKS**
  - `identity.jwks outcome=from_cache`: Keys were loaded from the JWKS cache.
  - `identity.jwks outcome=resolved`: Keys were fetched from the provider URI and then cached.

- **Email**
  - `identity.email.cache cache_hit=true` then `identity.email outcome=from_cache`: Email was taken from cache.
  - `identity.email.cache cache_hit=false` then `identity.email outcome=resolved`: Email was fetched via userinfo and then cached.

- **Profile**
  - `identity.profile.cache cache_hit=true` then `identity.profile outcome=from_cache`: Profile was taken from cache.
  - `identity.profile.cache cache_hit=false` then `identity.profile outcome=resolved`: Profile was loaded from the datastore and then cached.

Comparing cycles (e.g. first request vs later requests) shows whether later requests use the cache (more `from_cache` and `cache_hit=true`), which is expected after the first successful resolution.

## Example Workflow

1. Start the API with logs redirected to a file:
   ```bash
   RUST_LOG=info cargo run -p mycelium-api --bin myc_api -- --config settings/config.dev.for-docker.toml >> api.log 2>&1
   ```

2. Send a few authenticated or protected requests (e.g. with a bearer token) so that identity and profile resolution (and cache) are exercised.

3. Stop the API (or stop redirecting logs).

4. Run the validator:
   ```bash
   python3 scripts/python/evaluate_security_group_logs.py api.log --verbose
   ```

5. Check the result:
   - If **OK**, the log sequence is valid; use the verbose listing to confirm cache behaviour per cycle.
   - If **FAIL**, read the violation messages and fix the flow or logging so that every started stage has a matching outcome in the logs.

## Troubleshooting

- **No stage events found**: Ensure the log file contains lines with `stage=` (INFO level). Check that you are capturing stdout and that the log level is not too restrictive.
- **“identity.external started but no identity.external outcome=ok”**: The request may have failed before completing identity resolution (e.g. invalid token, network error). Check API and network logs.
- **“identity.jwks started but no identity.jwks outcome”**: JWKS fetch or parse may have failed. Look for ERROR/WARN lines around that timestamp in the raw log.
- **Multiple cycles**: Expected when you send multiple requests. Each cycle is one request’s identity/profile flow; use them to compare first request (often more `resolved`) vs later requests (often more `from_cache`).
