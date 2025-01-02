#!/bin/sh

set -e

export VAULT_ADDR=http://vault:8200

# give some time for Vault to start and be ready
sleep 3

# login with root token at $VAULT_ADDR
vault login root

# enable vault transit engine
vault secrets enable transit

# create key1 with type ed25519
vault write -f transit/keys/key1 type=ed25519

# create key2 with type ecdsa-p256
vault write -f transit/keys/key2 type=ecdsa-p256
