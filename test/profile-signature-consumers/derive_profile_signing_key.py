"""Derive the gateway profile signing key of one HMAC secret (test only).

Mirrors the gateway (`VersionedEd25519Signer::derive_from_secret`): the
Ed25519 seed is HKDF-SHA256(ikm = the secret's UTF-8 bytes, salt = empty,
info = "mycelium/profile-signing/ed25519/v1", length 32).

The secret is read from an environment variable, never from the command
line; nothing is printed. Both PEM files are written with private access.

Run: ``python derive_profile_signing_key.py <secret env var> <private pem> <public pem>``
"""
import os
import sys
from pathlib import Path

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.kdf.hkdf import HKDF

DERIVATION_INFO = b"mycelium/profile-signing/ed25519/v1"


def derive_private_key(secret: bytes) -> Ed25519PrivateKey:
    seed = HKDF(algorithm=hashes.SHA256(), length=32, salt=b"", info=DERIVATION_INFO).derive(secret)
    return Ed25519PrivateKey.from_private_bytes(seed)


def pem_pair(secret: bytes) -> tuple[bytes, bytes]:
    key = derive_private_key(secret)
    private = key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
                                serialization.NoEncryption())
    public = key.public_key().public_bytes(serialization.Encoding.PEM,
                                           serialization.PublicFormat.SubjectPublicKeyInfo)
    return private, public


def write_private(path: Path, content: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(content)


if __name__ == "__main__":
    variable, private_path, public_path = sys.argv[1:]
    secret = os.environ.get(variable, "")
    if not secret:
        raise SystemExit(f"{variable} is not set")
    private, public = pem_pair(secret.encode())
    write_private(Path(private_path), private)
    write_private(Path(public_path), public)
