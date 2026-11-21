"""Append adapter settings to a private generated test TOML (never print secrets)."""
import os
import sys
import tomllib
from pathlib import Path


def adapter_config(mode: str) -> str:
    if mode not in {"full", "postgres-only"}:
        raise ValueError("unsupported adapter mode")
    smtp_port = int(os.environ["PSIG_SMTP_PORT"])
    if not 0 < smtp_port < 65536:
        raise ValueError("invalid SMTP port")
    text = f'''
[diesel]
databaseUrl = {{ env = "PSIG_DATABASE_URL" }}
[smtp]
host = "localhost"
port = {smtp_port}
security = "starttls"
username = {{ env = "PSIG_SMTP_USER" }}
password = {{ env = "PSIG_SMTP_PASSWORD" }}
'''
    if mode == "full":
        port = int(os.environ["PSIG_REDIS_PORT"])
        if not 0 < port < 65536:
            raise ValueError("invalid Redis port")
        text += f'''
[redis]
protocol = "redis"
hostname = "127.0.0.1"
port = {port}
password = {{ env = "PSIG_REDIS_PASSWORD" }}
'''
    tomllib.loads(text)
    return text


if __name__ == "__main__":
    path = Path(sys.argv[1])
    text = path.read_text() + adapter_config(sys.argv[2])
    tomllib.loads(text)
    path.write_text(text, encoding="utf-8")
