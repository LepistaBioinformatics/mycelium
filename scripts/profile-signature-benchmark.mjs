// Bounded keep-alive HTTP load client. Reads credentials from a private file.
import http from "node:http"
import { readFileSync, appendFileSync } from "node:fs"
import { performance } from "node:perf_hooks"
const config = JSON.parse(readFileSync(process.argv[2], "utf8"))
const concurrency = Number(process.argv[3])
const repetition = Number(process.argv[4])
if (![1, 8, 32].includes(concurrency)) throw new Error("invalid concurrency")
const agent = new http.Agent({ keepAlive: true, maxSockets: concurrency })
const samples = []
function processUsage(pid) {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, "utf8").split(") ")[1].split(" ")
    const status = readFileSync(`/proc/${pid}/status`, "utf8")
    return { pid, cpuTicks: Number(stat[11]) + Number(stat[12]),
      rssKiB: Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1] ?? 0) }
  } catch { return { pid, unavailable: true } }
}
async function once(collect) {
  const start = performance.now()
  let firstByte = start
  const result = await new Promise((resolve) => {
    const req = http.get(config.url, { agent, headers: config.headers }, (res) => {
      firstByte = performance.now()
      const chunks = []
      res.on("data", (chunk) => chunks.push(chunk))
      res.on("error", () => resolve({ status: 0, outcome: "response_error" }))
      res.on("end", () => {
        const downloaded = performance.now()
        let outcome = "ok"
        try { if (JSON.parse(Buffer.concat(chunks)).accId !== config.account) outcome = "wrong_account" }
        catch { outcome = "invalid_response" }
        if (res.statusCode !== 200) outcome = "http_error"
        resolve({ status: res.statusCode, outcome, requestMs: firstByte - start,
          downloadMs: downloaded - firstByte, parseMs: performance.now() - downloaded })
      })
    })
    req.setTimeout(10000, () => req.destroy(new Error("timeout")))
    req.on("error", (error) => resolve({ status: 0, outcome: error.message === "timeout" ? "timeout" : "network_error" }))
  })
  if (collect) samples.push({ ...result, durationMs: performance.now() - start })
}
async function load(durationMs, collect, minimum = 0) {
  const start = performance.now()
  const end = start + durationMs
  const maxEnd = start + 30000
  await Promise.all(Array.from({ length: concurrency }, async () => {
    do { await once(collect) }
    while (performance.now() < end || (samples.length < minimum && performance.now() < maxEnd))
  }))
  return performance.now() - start
}
function percentile(values, rank) {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.max(0, Math.ceil(rank * sorted.length) - 1)] ?? null
}
await load(1000, false)
const before = config.pids.map(processUsage)
const peakRss = before.map((usage) => usage.rssKiB ?? 0)
const monitor = setInterval(() => config.pids.forEach((pid, index) => {
  peakRss[index] = Math.max(peakRss[index], processUsage(pid).rssKiB ?? 0)
}), 100)
const clientCpuBefore = process.cpuUsage()
const elapsedMs = await load(5000, true, 200)
const after = config.pids.map(processUsage)
clearInterval(monitor)
const clientCpu = process.cpuUsage(clientCpuBefore)
agent.destroy()
const successful = samples.filter((sample) => sample.outcome === "ok")
const latencies = successful.map((sample) => sample.durationMs)
const result = { schemaVersion: 1, mode: config.mode, framework: config.framework,
  state: config.state, dataset: config.dataset, profileBytes: config.profileBytes,
  profileJsonBytes: config.profileJsonBytes, pathKind: config.pathKind ?? "gateway",
  signatureBytes: config.signatureBytes, datasetSha256: config.datasetSha256,
  concurrency, repetition, warmupMs: 1000, elapsedMs, samples,
  count: samples.length, errors: samples.length - successful.length,
  throughputPerSecond: successful.length * 1000 / elapsedMs,
  p50Ms: percentile(latencies, 0.5), p95Ms: percentile(latencies, 0.95),
  p99Ms: percentile(latencies, 0.99),
  minMs: latencies.reduce((value, item) => Math.min(value, item), Infinity),
  maxMs: latencies.reduce((value, item) => Math.max(value, item), 0),
  percentileMethod: "nearest-rank", before, after, sampledPeakRssKiB: peakRss,
  cpuSeconds: after.map((usage, index) => (usage.cpuTicks - before[index].cpuTicks) / config.cpuTicksPerSecond),
  clientCpuSeconds: (clientCpu.user + clientCpu.system) / 1e6,
  clientRssBytes: process.memoryUsage().rss }
appendFileSync(config.output, JSON.stringify(result) + "\n")
console.log(`BENCH ${config.mode} ${config.state} ${config.dataset} c=${concurrency} rep=${repetition}: n=${result.count} errors=${result.errors} p50=${result.p50Ms?.toFixed(3)}ms`)
if (result.errors || successful.length < 200) process.exitCode = 1
