"""Hash the executable/test sources used by a local coordinated run."""
import hashlib
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

EXCLUDED = {".git", "node_modules", ".venv", "venv", "target", "dist", "build", "__pycache__",
            ".pytest_cache", ".mypy_cache", ".ruff_cache", "book", "book-pt-BR", "evidence"}
EXTENSIONS = {".rs", ".py", ".ts", ".js", ".mjs", ".cjs", ".json", ".toml", ".lock", ".sh", ".yml", ".yaml", ".sql",
              ".go", ".mod", ".sum"}
COMPONENTS = ("mycelium-api-gateway", "mycelium-sdk-py", "mycelium-sdk-js", "mycelium-sdk-go")


def source_files(root):
    for component in COMPONENTS:
        base = root / "modules" / component
        for current, directories, files in os.walk(base):
            directories[:] = sorted(name for name in directories if name not in EXCLUDED and not name.startswith(".env"))
            for name in sorted(files):
                path = Path(current) / name
                if path.suffix in EXTENSIONS and not name.startswith(".env"):
                    yield path
    yield root / "scripts/validate-profile-signature.sh"
    yield root / ".github/workflows/profile-signature-integration.yml"
    yield root / ".gitmodules"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True, timeout=10).strip()


def collect(root, output, binaries):
    files = [dict(path=str(p.relative_to(root)), size=p.stat().st_size, sha256=digest(p))
             for p in sorted(source_files(root))]
    repositories = [root] + [root / "modules" / name for name in COMPONENTS]
    bases = {str(p.relative_to(root)): command("git", "rev-parse", "HEAD", cwd=p) for p in repositories}
    artifacts = {mode: dict(path=str(binaries / mode / "myc-api"), sha256=digest(binaries / mode / "myc-api"),
                           features=mode, profile="release") for mode in ("standalone", "postgres-only", "full")}
    manifest = dict(schemaVersion=1, createdUtc=datetime.now(timezone.utc).isoformat(), bases=bases,
                    files=files, binaries=artifacts, platform=platform.platform(), python=sys.version,
                    rust=command("rustc", "--version"), node=command("node", "--version"),
                    cargo=command("cargo", "--version"), mdbook=command("mdbook", "--version"),
                    cpuTicksPerSecond=os.sysconf("SC_CLK_TCK"),
                    cpu=[line.strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")][:1],
                    memory=[line for line in Path("/proc/meminfo").read_text().splitlines() if line.startswith(("MemTotal:", "SwapTotal:"))])
    output.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"SOURCE MANIFEST {len(files)} files SHA256 {digest(output)}")


if __name__ == "__main__":
    collect(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))
