"""Synthetic FastAPI downstream for the profile signature E2E (test only).

Uses the local SDK checkout (editable install). Routes:

- ``GET /health``: no SDK, always 200.
- ``GET /whoami``: SDK dependency ``get_profile_from_header_required``;
  verifies the signature when ``MYC_PROFILE_PUBLIC_KEY_*`` is set.
- ``GET /capture``: no SDK; returns the identity headers it received so the
  runner can replay or tamper with a real gateway-issued pair.
- ``GET /stats``: how many times the ``/whoami`` handler actually ran.

Run: ``python python_app.py <port>``
"""

import sys
import os
import secrets
from contextlib import asynccontextmanager

import uvicorn
from fastapi import Depends, FastAPI, Request, HTTPException

from myc_http_tools import configure_profile_signature
from myc_http_tools.fastapi import get_profile_from_header_required

CALLS = {"whoami": 0}


@asynccontextmanager
async def lifespan(_app):
    configure_profile_signature()
    yield


app = FastAPI(lifespan=lifespan, openapi_url=None)


@app.get("/openapi.json")
def schema():
    return {"openapi": "3.0.3", "info": {"title": "PSIG synthetic consumer", "version": "1"},
            "paths": {"/whoami": {"get": {"operationId": "psigWhoami",
                      "responses": {"200": {"description": "Account"}}}}}}


@app.get("/health")
def health():
    return {"status": "ok", "runId": os.environ.get("PSIG_RUN_ID"), "verification": bool(os.environ.get("MYC_PROFILE_PUBLIC_KEY_1"))}


@app.get("/whoami", operation_id="psigWhoami")
def whoami(profile=Depends(get_profile_from_header_required)):
    CALLS["whoami"] += 1
    return {"framework": "fastapi", "accId": str(profile.acc_id)}


@app.get("/capture")
def capture(request: Request):
    control = os.environ.get("PSIG_CONTROL_TOKEN", "")
    if not control or not secrets.compare_digest(request.headers.get("x-psig-control", ""), control):
        raise HTTPException(403)
    return {
        "profile": request.headers.get("x-mycelium-profile"),
        "signature": request.headers.get("x-mycelium-profile-signature"),
    }


@app.get("/public")
@app.get("/authenticated")
def inspect_headers(request: Request):
    return {"profilePresent": "x-mycelium-profile" in request.headers,
            "signaturePresent": "x-mycelium-profile-signature" in request.headers}


@app.get("/stats")
def stats():
    return CALLS


if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=int(sys.argv[1]), log_level="warning")
