# Tutorial: Docker

Run the Labelize HTTP service (REST API + web playground) in a container.
Pre-built multi-arch images (`linux/amd64` + `linux/arm64`) are published on
every release.

## 1. Run the service

```bash
# Docker Hub
docker run -p 8080:8080 goodboy008/labelize:latest

# or GitHub Container Registry
docker run -p 8080:8080 ghcr.io/goodboy008/labelize:latest
```

Available tags:

| Tag | Meaning |
|-----|---------|
| `latest` | newest stable release |
| `1.6.0` / `1.6` / `1` | pinned release versions |
| `edge` | current `main` branch |

## 2. Verify it's up

```bash
curl http://localhost:8080/health
# {"status":"ok"}
```

Open <http://localhost:8080/> in a browser for the interactive playground —
paste ZPL/EPL, pick a label size, download PNG or PDF.

## 3. Convert a label

```bash
curl -X POST http://localhost:8080/convert \
  -H "Content-Type: application/zpl" \
  -d '^XA^FO50,50^A0N,40,40^FDHello Docker^FS^XZ' \
  -o label.png
```

The `Content-Type` header selects the parser (`application/zpl` or
`application/epl`). Query parameters (`width`, `height`, `dpmm`, `output`,
`antialias`) are the same as the native service — see the
[HTTP service tutorial](http-service.md#convert-endpoint) for the full table.

## 4. Run with Compose

A `docker-compose.yaml` ships in the repo root:

```bash
git clone https://github.com/GOODBOY008/labelize
cd labelize
docker compose up -d --build
```

This builds the image from source (useful for custom forks or offline
environments) and starts it on port 8080.

## 5. Production notes

- The container listens on `0.0.0.0:8080`; map `-p 8080:8080` or adjust to
  taste. Put a TLS-terminating reverse proxy in front for public exposure.
- The service is stateless — scale horizontally behind a load balancer if
  needed.
- Pin a version tag (`docker.io/goodboy008/labelize:1.6.0`) rather than
  `latest` for reproducible deployments.

## 6. What's next

- [HTTP service tutorial](http-service.md) — full API reference
- [CLI tutorial](cli.md) — one-off conversions without a server
