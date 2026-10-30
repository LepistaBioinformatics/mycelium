// Synthetic Express/Fastify downstream for the profile signature E2E (test
// only). Loads the SDK from the local build (`dist/`) of the checkout given in
// MYC_SDK_JS_DIR, never from the published package.
//
// Routes: GET /health (no SDK), GET /whoami (SDK verification when
// MYC_PROFILE_PUBLIC_KEY_* is set), GET /capture (no SDK, echoes identity
// headers), GET /stats (handler call count).
//
// Run: node javascript_app.mjs <express|fastify> <port>

import { timingSafeEqual } from "node:crypto"
import { createRequire } from "node:module"
import { join } from "node:path"
import { pathToFileURL } from "node:url"

const [framework, rawPort] = process.argv.slice(2)
const port = Number(rawPort)
const sdkDir = process.env.MYC_SDK_JS_DIR
if (!sdkDir) throw new Error("MYC_SDK_JS_DIR is required")

const requireFromSdk = createRequire(join(sdkDir, "package.json"))
const sdkEntry = (name) =>
  import(pathToFileURL(join(sdkDir, "dist", name, "index.js")).href)

const calls = { whoami: 0 }

const health = () => ({ status: "ok", runId: process.env.PSIG_RUN_ID,
  verification: Boolean(process.env.MYC_PROFILE_PUBLIC_KEY_1) })
const inspect = (headers) => ({ profilePresent: Boolean(headers["x-mycelium-profile"]),
  signaturePresent: Boolean(headers["x-mycelium-profile-signature"]) })
const openapi = { openapi: "3.0.3", info: { title: "PSIG synthetic consumer", version: "1" },
  paths: { "/whoami": { get: { operationId: "psigWhoami", responses: { "200": { description: "Account" } } } } } }
function allowed(headers) {
  const actual = Buffer.from(headers["x-psig-control"] ?? "")
  const expected = Buffer.from(process.env.PSIG_CONTROL_TOKEN ?? "")
  return expected.length > 0 && actual.length === expected.length && timingSafeEqual(actual, expected)
}
function capture(headers) {
  return {
    profile: headers["x-mycelium-profile"] ?? null,
    signature: headers["x-mycelium-profile-signature"] ?? null,
  }
}

async function startExpress() {
  const express = requireFromSdk("express")
  const { getProfileFromHeaderRequired } = await sdkEntry("express")
  const app = express()

  app.get("/health", (_req, res) => res.json(health()))
  app.get("/openapi.json", (_req, res) => res.json(openapi))
  for (const path of ["/public", "/authenticated"]) app.get(path, (req, res) => res.json(inspect(req.headers)))
  app.get("/capture", (req, res) => allowed(req.headers) ? res.json(capture(req.headers)) : res.sendStatus(403))
  app.get("/stats", (_req, res) => res.json(calls))
  app.get("/whoami", getProfileFromHeaderRequired(), (req, res) => {
    calls.whoami += 1
    res.json({ framework: "express", accId: req.profile?.accId ?? null })
  })

  const server = app.listen(port, "127.0.0.1")
  for (const signal of ["SIGTERM", "SIGINT"]) process.once(signal, () => server.close(() => process.exit(0)))
}

async function startFastify() {
  const Fastify = requireFromSdk("fastify")
  const { fastifyProfilePlugin } = await sdkEntry("fastify")
  const app = Fastify()

  app.get("/health", async () => health())
  app.get("/openapi.json", async () => openapi)
  for (const path of ["/public", "/authenticated"]) app.get(path, async (req) => inspect(req.headers))
  app.get("/capture", async (request, reply) => {
    if (!allowed(request.headers)) return reply.code(403).send()
    return capture(request.headers)
  })
  app.get("/stats", async () => calls)
  await app.register(async (scoped) => {
    await scoped.register(fastifyProfilePlugin, { environment: "production" })
    scoped.get("/whoami", async (request) => {
      calls.whoami += 1
      return { framework: "fastify", accId: request.profile?.accId ?? null }
    })
  })

  await app.listen({ port, host: "127.0.0.1" })
  for (const signal of ["SIGTERM", "SIGINT"]) process.once(signal, async () => { await app.close(); process.exit(0) })
}

if (framework === "express") await startExpress()
else if (framework === "fastify") await startFastify()
else throw new Error(`unknown framework ${framework}`)
