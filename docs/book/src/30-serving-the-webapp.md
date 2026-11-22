# Serving the Webapp

The [Mycelium webapp](https://github.com/LepistaBioinformatics/mycelium-webapp)
is the user and operator interface: sign-in, accounts, tenants, guests and
federation. It can run as its own service (nginx in front of the static build),
or the gateway can serve it under `/_app/`. Serving it from the gateway means
one process, one origin, no CORS setup and no second deployment.

> **Opt-in.** Serving the webapp needs the `webapp` cargo feature and an
> `[api.webapp]` section. Without either, `/_app/*` goes to the downstream
> router like any other path and nothing changes.

---

## How it works

- The gateway serves the files of a webapp build (its `dist/` directory) from
  disk, under the fixed prefix `/_app/`. The prefix cannot be changed, because
  the build itself is made for it.
- Any path under `/_app/` that is not a file of the build and has no file
  extension is answered with `index.html`, so deep links such as
  `/_app/dashboard/federation` survive a page reload. A missing file with an
  extension, such as `/_app/assets/missing.js`, is a 404.
- `/_app/*` is public, as the files of any website are. Requests under it do
  not go through the downstream router or authentication. The webapp then
  calls the gateway API (`/_adm/...`) on the same origin, with the user's
  token.
- `index.html` and the service worker are never cached, so users get a new
  build on their next visit. The hashed assets are cached for a year. Every
  answer carries `X-Frame-Options: SAMEORIGIN`,
  `X-Content-Type-Options: nosniff` and
  `Referrer-Policy: strict-origin-when-cross-origin`.
- Requests that try to leave the build directory are never answered with a
  file outside it. That covers `..` segments, encoded traversals and symlinks
  pointing out.

---

## 1. Build the gateway with the feature

Add `webapp` to the features of your build mode:

```bash
# Full mode
cargo build --release -p mycelium-api --features webapp

# Postgres-only mode
cargo build --release -p mycelium-api \
  --no-default-features --features postgres-only,webapp

# Standalone mode
cargo build --release -p mycelium-api \
  --no-default-features --features standalone,webapp
```

The published Docker images are built without it. Build your own image with
the feature to serve the webapp from a container.

## 2. Build the webapp for `/_app/`

From a checkout of the webapp, at the version that matches your gateway:

```bash
yarn install --frozen-lockfile
VITE_MYCELIUM_API_URL= VITE_BASE_PATH=/_app/ yarn build
```

- `VITE_BASE_PATH=/_app/` makes every asset, route and the service worker
  resolve under `/_app/`.
- The empty `VITE_MYCELIUM_API_URL=` makes the webapp call the API on its own
  origin. Keep it empty even when a local `.env` file sets it, because the
  value from the shell wins.

The output is `dist/`. The same build works on any gateway, since it holds no
URL and no client id.

## 3. Configure the gateway

Point `distDir` to the build and name the hosted login client the webapp
signs in with:

```toml
[api.webapp]
distDir = "/srv/mycelium-webapp/dist"
hostedLoginClientId = "webapp-01"
```

`hostedLoginClientId` is optional. When set, `GET /app-config` publishes it,
and the webapp offers the gateway-hosted login with that client. The client
must exist and accept the webapp's callback:

```toml
[auth.hostedLogin.define]
clients = [
  { clientId = "webapp-01", redirectUris = ["https://myc-01.example.com/_app/callback"] },
]
```

The callback is your gateway's public origin followed by `/_app/callback`.

## 4. Open it

Start the gateway and open `https://myc-01.example.com/_app/`. A fresh
instance asks for its first staff account to be claimed before anyone can
sign in. Claim it at `/_adm/instance/bootstrap` with
`core.accountLifeCycle.staffBootstrapSecret`.

### With Docker

Mount the build into a gateway image built with the feature, and point
`distDir` at the mount:

```bash
docker run -p 8080:8080 \
  -v ./dist:/srv/mycelium-webapp/dist:ro \
  -v ./config.toml:/home/config.toml:ro \
  my-registry/mycelium-api-webapp
```

`my-registry/mycelium-api-webapp` stands for your own image of the gateway,
built with `--features webapp`.

---

## Startup checks

The gateway refuses to start, with an error naming the problem and the path,
when `[api.webapp]` is set and:

- the binary was built without the `webapp` feature,
- `distDir` does not exist or cannot be read, or
- `distDir` has no `index.html`.

It also logs a warning for each downstream route configured under `/_app`,
since those routes cannot be reached while the webapp is served.

---

## Troubleshooting

| Symptom | Cause |
| --- | --- |
| Blank page, assets answer 404 | The webapp was built without `VITE_BASE_PATH=/_app/`. Rebuild it. |
| API calls go to another host | `VITE_MYCELIUM_API_URL` was set during the build, possibly from a `.env` file. Rebuild with it empty. |
| No hosted login option | `hostedLoginClientId` is not set, so `GET /app-config` has no `hostedLoginClientId`. |
| Login fails after the gateway's sign-in page | The client's `redirectUris` lack `<origin>/_app/callback`. |
| `/_app/` answers like a downstream route | The gateway was built without the `webapp` feature, or `[api.webapp]` is missing. |

To upgrade the webapp, replace the contents of `distDir`. No restart is
needed: the next page load picks up the new `index.html`.
