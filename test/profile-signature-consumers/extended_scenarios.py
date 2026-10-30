"""Additional real HTTP assertions. Credentials and captured pairs stay private."""
import base64
import hashlib
import http.client
import json
import os
import sys
import time
from pathlib import Path

import jwt

runtime, mode = Path(sys.argv[1]), sys.argv[2]
api = int(os.environ["API_PORT"])
bearer = {"Authorization": "Bearer " + os.environ["JWT"]}
control = {"x-psig-control": os.environ["PSIG_CONTROL_TOKEN"]}
account = os.environ["ACC_ID"]
number = 0
scenario_started = time.perf_counter()


def request(port, path, headers=None, body=None, duplicate=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    try:
        connection.putrequest("POST" if body is not None else "GET", path)
        for key, value in (headers or {}).items():
            connection.putheader(key, value)
        if duplicate:
            connection.putheader(*duplicate)
        encoded = json.dumps(body).encode() if body is not None else None
        if encoded is not None:
            connection.putheader("Content-Type", "application/json")
            connection.putheader("Content-Length", str(len(encoded)))
        connection.endheaders(encoded)
        response = connection.getresponse()
        raw = response.read()
        is_json = "json" in response.getheader("Content-Type", "")
        return response.status, json.loads(raw) if raw and is_json else None
    finally:
        connection.close()


def record(label, status, expected, before=None, after=None):
    global number, scenario_started
    assert status == expected, f"{label}: expected {expected}, got {status}"
    if before is not None and expected != 200:
        assert before == after, f"{label}: rejected request executed handler"
    number += 1
    item = dict(id=f"extended-{number:02}", mode=mode, label=label, outcome="pass",
                observedStatus=status, expectedStatus=expected, handlerBefore=before, handlerAfter=after,
                durationMs=(time.perf_counter()-scenario_started)*1000)
    with (runtime / "results.jsonl").open("a") as stream:
        stream.write(json.dumps(item) + "\n")
    print("  PASS " + label, flush=True)
    scenario_started = time.perf_counter()


def capture():
    status, pair = request(api, "/py-svc/capture", bearer | control)
    assert status == 200 and pair["signature"]
    return pair


def claims(pair):
    return jwt.decode(pair["signature"], (runtime / "public-v1.pem").read_text(),
                      algorithms=["EdDSA"], options={"verify_aud": False})


pair = capture()
payload = claims(pair)
expected_hash = base64.urlsafe_b64encode(hashlib.sha256(pair["profile"].encode()).digest()).decode().rstrip("=")
assert payload["ph"] == expected_hash and payload["aud"] == "py-svc"
record("real gateway JWS verifies with exact profile hash and service audience", 200, 200)
for path in ("public", "authenticated"):
    headers = {"x-mycelium-profile": "forged", "x-mycelium-profile-signature": "forged"}
    if path == "authenticated":
        headers |= bearer
    status, value = request(api, "/py-svc/" + path, headers)
    assert value == {"profilePresent": False, "signaturePresent": False}
    record(path + " strips inbound identity and sends no profile/signature", status, 200)
status, _ = request(api, "/py-svc/authenticated")
record("authenticated route requires identity", status, 401)
status, _ = request(api, "/py-svc/roles", bearer)
record("ProtectedByRoles refuses missing role before forwarding", status, 403)

# Create the connection string through the real public RPC, never by forging HMAC.
status, value = request(api, "/_adm/rpc", bearer, {"jsonrpc": "2.0", "id": 1,
    "method": "beginners.tokens.create", "params": {"name": "psig-e2e", "expiration": 600,
    "tenantId": None, "serviceAccountId": None, "roles": None}})
assert status == 200 and "result" in value, "connection string RPC failed"
connection_string = value["result"]["connectionString"]
for service in ("py-svc", "express-svc", "fastify-svc", "go-svc", "rust-svc"):
    status, value = request(api, f"/{service}/whoami", {"x-mycelium-connection-string": connection_string})
    assert value["accId"] == account
    record(service + " verifies signed profile from real connection string", status, 200)

# Consecutive misses/hits must still receive a newly timestamped signature.
first = capture()
deadline = time.monotonic() + 2
while int(time.time()) == claims(first)["iat"] and time.monotonic() < deadline:
    time.sleep(0.02)
second = capture()
assert first["profile"] == second["profile"]
assert claims(second)["iat"] > claims(first)["iat"] and first["signature"] != second["signature"]
record("cached profile keeps bytes but receives a fresh signature on forwarding", 200, 200)
log = (runtime / "api.log").read_text()
assert "Profile cache: hit" in log and "Profile cache: miss" in log
record("gateway trace confirms both real profile cache miss and hit", 200, 200)

audiences = {"Express": "express-svc", "Go": "go-svc", "Rust": "rust-svc"}
for framework, port in (("FastAPI", int(os.environ["PY_PORT"])),
                        ("Express", int(os.environ["EXPRESS_PORT"])),
                        ("Fastify", int(os.environ["FASTIFY_PORT"])),
                        ("Go", int(os.environ["GO_PORT"])),
                        ("Rust", int(os.environ["RUST_PORT"]))):
    audience = audiences.get(framework, "py-svc")
    def signed(changes=None, kid="1"):
        value = payload | {"aud": audience, "iat": int(time.time())} | (changes or {})
        return jwt.encode(value, (runtime / "private-v1.pem").read_text(), algorithm="EdDSA", headers={"kid": kid})
    valid = signed()
    headers = {"x-mycelium-profile": pair["profile"], "x-mycelium-profile-signature": valid}
    negatives = [
        ("expired", headers | {"x-mycelium-profile-signature": signed({"iat": int(time.time()) - 120})}, None),
        ("future", headers | {"x-mycelium-profile-signature": signed({"iat": int(time.time()) + 120})}, None),
        ("unknown kid", headers | {"x-mycelium-profile-signature": signed(kid="99")}, None),
        ("oversized signature", headers | {"x-mycelium-profile-signature": "A" * 4097}, None),
        ("duplicate profile", headers, ("x-mycelium-profile", pair["profile"])),
        ("duplicate signature", headers, ("x-mycelium-profile-signature", valid)),
    ]
    for label, candidate, duplicate in negatives:
        before = request(port, "/stats")[1]["whoami"]
        status, _ = request(port, "/whoami", candidate, duplicate=duplicate)
        after = request(port, "/stats")[1]["whoami"]
        record(framework + " " + label + " rejected before handler", status, 403, before, after)
    status, _ = request(port, "/capture")
    record(framework + " capture requires private control token", status, 403)

status, tools = request(api, "/mcp", bearer, {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
assert status == 200 and "result" in tools
names = [tool["name"] for tool in tools["result"]["tools"] if "py-svc" in tool["name"] or "py_svc" in tool["name"]]
assert names, "synthetic OpenAPI tool absent from MCP registry"
before = request(int(os.environ["PY_PORT"]), "/stats")[1]["whoami"]
status, response = request(api, "/mcp", bearer, {"jsonrpc": "2.0", "id": 3,
    "method": "tools/call", "params": {"name": names[0], "arguments": {}}})
assert "result" in response and not response["result"].get("isError"), "MCP loopback returned error"
after = request(int(os.environ["PY_PORT"]), "/stats")[1]["whoami"]
assert after == before + 1 and account in json.dumps(response["result"])
record("MCP loopback signs profile accepted by real FastAPI SDK handler", status, 200, before, after)
