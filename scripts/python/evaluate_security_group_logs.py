#!/usr/bin/env python3
"""
Validate security group / identity / profile log sequence in Mycelium API log files.

Parses tracing logs (tracing format with optional multi-line entries), extracts
events that contain stage= and outcome=, and validates that the expected sequences
of stages occur (e.g. identity.jwks -> outcome, identity.email -> outcome,
identity.profile -> outcome).

Usage:
  python evaluate_security_group_logs.py <log_file>
  python evaluate_security_group_logs.py 2026-02-03T20:45:41.log
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path


# Regex to match a log line that starts with optional spaces + ISO timestamp +
# level
LOG_ENTRY_START = re.compile(
    r"^\s*(\d{4}-\d{2}-\d{2}T[\d:.]+Z)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+"
)
# Extract stage="..." or stage: "..."
STAGE_RE = re.compile(r'stage:\s*["\']([^"\']+)["\']')
# Extract outcome="..." or outcome: "..."
OUTCOME_RE = re.compile(r'outcome:\s*["\']([^"\']+)["\']')
# Extract cache_hit=true/false
CACHE_HIT_RE = re.compile(r"cache_hit:\s*(true|false)")
# Message part after the module (e.g. "Resolving JWKS, stage: ...")
MESSAGE_RE = re.compile(r"(?:TRACE|DEBUG|INFO|WARN|ERROR)\s+\S+:\s*(.+)")


@dataclass
class StageEvent:
    """A single log event that carries stage (and optionally outcome)."""

    timestamp: str
    level: str
    message: str
    stage: str
    outcome: str | None = None
    cache_hit: bool | None = None
    raw_line: str = ""

    def __str__(self) -> str:
        parts = [f"{self.timestamp} {self.stage}"]
        if self.outcome:
            parts.append(f"outcome={self.outcome}")
        if self.cache_hit is not None:
            parts.append(f"cache_hit={str(self.cache_hit).lower()}")
        return " ".join(parts)


def parse_log_file(path: Path) -> list[StageEvent]:
    """Read log file and return list of stage events in order."""
    text = path.read_text(encoding="utf-8", errors="replace")
    events: list[StageEvent] = []
    current_entry_lines: list[str] = []
    current_timestamp = ""
    current_level = ""

    def flush_entry() -> None:
        nonlocal current_entry_lines, current_timestamp, current_level
        if not current_entry_lines:
            return
        full = " ".join(current_entry_lines)
        stage_m = STAGE_RE.search(full)
        if not stage_m:
            current_entry_lines = []
            return
        stage = stage_m.group(1)
        outcome_m = OUTCOME_RE.search(full)
        outcome = outcome_m.group(1) if outcome_m else None
        cache_m = CACHE_HIT_RE.search(full)
        cache_hit = None
        if cache_m:
            cache_hit = cache_m.group(1).lower() == "true"
        msg_m = MESSAGE_RE.search(full)
        message = msg_m.group(1).strip() if msg_m else full
        events.append(
            StageEvent(
                timestamp=current_timestamp,
                level=current_level,
                message=message,
                stage=stage,
                outcome=outcome,
                cache_hit=cache_hit,
                raw_line=current_entry_lines[0][:80] if current_entry_lines else "",
            )
        )
        current_entry_lines = []

    for line in text.splitlines():
        if not line.strip():
            flush_entry()
            continue
        match = LOG_ENTRY_START.match(line)
        if match:
            flush_entry()
            current_timestamp = match.group(1)
            current_level = match.group(2)
            current_entry_lines = [line]
        else:
            if current_entry_lines and (line.startswith(" ") or line.startswith("\t")):
                current_entry_lines.append(line)

    flush_entry()
    return events


def validate_sequences(events: list[StageEvent]) -> tuple[bool, list[str]]:
    """
    Validate that stage events follow expected sequences.
    Returns (all_ok, list of violation messages).
    """
    violations: list[str] = []
    i = 0
    while i < len(events):
        e = events[i]
        # identity.jwks: expect start then later outcome (from_cache | resolved)
        if e.stage == "identity.jwks" and e.outcome is None:
            j = i + 1
            while (
                j < len(events)
                and events[j].stage == "identity.jwks"
                and events[j].outcome is None
            ):
                j += 1
            if (
                j < len(events)
                and events[j].stage == "identity.jwks"
                and events[j].outcome in ("from_cache", "resolved")
            ):
                i = j + 1
                continue
            if j >= len(events) or events[j].stage != "identity.jwks":
                violations.append(
                    f"At {e.timestamp}: identity.jwks started but no identity.jwks outcome (from_cache/resolved) found"
                )
            else:
                violations.append(
                    f"At {e.timestamp}: identity.jwks started but next identity.jwks has unexpected outcome"
                )
            i += 1
            continue

        # identity.email (with outcome): valid terminal; no extra check
        if e.stage == "identity.email" and e.outcome in (
            "from_cache",
            "resolved",
            "from_token",
        ):
            i += 1
            continue

        # identity.profile: start then optional cache then outcome
        if e.stage == "identity.profile" and e.outcome is None:
            j = i + 1
            found_outcome = False
            while j < len(events):
                n = events[j]
                if n.stage == "identity.profile.cache":
                    j += 1
                    continue
                if n.stage == "identity.profile" and n.outcome in (
                    "from_cache",
                    "resolved",
                ):
                    found_outcome = True
                    i = j + 1
                    break
                break
            if not found_outcome:
                violations.append(
                    f"At {e.timestamp}: identity.profile started but no identity.profile outcome (from_cache/resolved) found"
                )
                i += 1
            continue

        # identity.external: started then later outcome=ok
        if e.stage == "identity.external" and e.outcome is None:
            j = i + 1
            found_ok = False
            while j < len(events):
                if events[j].stage == "identity.external" and events[j].outcome == "ok":
                    found_ok = True
                    i = j + 1
                    break
                j += 1
            if not found_ok:
                violations.append(
                    f"At {e.timestamp}: identity.external started but no identity.external outcome=ok found"
                )
                i += 1
            continue

        # router.*: started then outcome=ok or outcome=error (optional; only validated when present)
        if e.stage.startswith("router.") and e.outcome is None:
            j = i + 1
            found_outcome = False
            while j < len(events):
                if events[j].stage == e.stage and events[j].outcome in ("ok", "error"):
                    found_outcome = True
                    i = j + 1
                    break
                j += 1
            if not found_outcome:
                violations.append(
                    f"At {e.timestamp}: {e.stage} started but no outcome (ok/error) found"
                )
                i += 1
            continue

        i += 1

    return (len(violations) == 0, violations)


def group_events_into_cycles(events: list[StageEvent]) -> list[list[StageEvent]]:
    """
    Group stage events into cycles. A new cycle starts at each identity.external
    (no outcome), i.e. start of a new request's identity resolution.
    """
    cycles: list[list[StageEvent]] = []
    current: list[StageEvent] = []

    for ev in events:
        if ev.stage == "identity.external" and ev.outcome is None:
            if current:
                cycles.append(current)
            current = [ev]
        else:
            current.append(ev)

    if current:
        cycles.append(current)

    return cycles


def print_report(
    events: list[StageEvent], all_ok: bool, violations: list[str], verbose: bool
) -> None:
    """Print validation report and optional event list (with cycles separated when verbose)."""
    print("=" * 60)
    print("Security group / identity / profile log validation")
    print("=" * 60)
    print(f"Total stage events found: {len(events)}")
    print()

    if verbose and events:
        cycles = group_events_into_cycles(events)
        print(f"Stage sequence ({len(cycles)} cycle(s)):")
        print()
        for idx, cycle in enumerate(cycles, start=1):
            print("  --- Cycle", idx, f"({len(cycle)} events) ---")
            for ev in cycle:
                print(f"    {ev}")
            if idx < len(cycles):
                print()
        print()

    if all_ok:
        print("Result: OK – expected stage sequences are satisfied.")
    else:
        print("Result: FAIL – violations found:")
        for v in violations:
            print(f"  - {v}")
    print("=" * 60)


def main() -> int:
    if len(sys.argv) < 2:
        print(
            "Usage: python evaluate_security_group_logs.py <log_file> [--verbose]",
            file=sys.stderr,
        )
        return 2
    path = Path(sys.argv[1])
    verbose = "--verbose" in sys.argv or "-v" in sys.argv
    if not path.exists():
        print(f"Error: file not found: {path}", file=sys.stderr)
        return 1
    events = parse_log_file(path)
    all_ok, violations = validate_sequences(events)
    print_report(events, all_ok, violations, verbose)
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
