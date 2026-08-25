# Docker Image Publishing

The `Docker` workflow (`.github/workflows/docker.yml`) builds the container image
and publishes multi-arch manifests to **Docker Hub** and the **GitHub Container
Registry (GHCR)**.

## Maintainer setup

GHCR works out of the box — it authenticates with the built-in `GITHUB_TOKEN`.
Docker Hub needs two settings, both under
**Settings → Secrets and variables → Actions**:

| Name | Kind | Value |
|------|------|-------|
| `DOCKERHUB_USERNAME` | **Variable** | Docker Hub account name |
| `DOCKERHUB_TOKEN` | **Secret** | Docker Hub access token (Account Settings → Personal access tokens, `Read & Write` scope) |

> [!IMPORTANT]
> The username must be a repository **variable**, not a secret. GitHub redacts
> job outputs that contain a secret value and passes them on as empty strings.
> The account name is a substring of the image names this workflow computes
> (`ghcr.io/goodboy008/labelize` contains `goodboy008`), so storing it as a
> secret would silently blank out those outputs and break publishing to *both*
> registries. Only the token is sensitive and belongs in Secrets.

Until both exist the workflow still succeeds — it logs a notice and publishes to
GHCR only. This keeps forks and pre-setup runs green instead of failing on
missing credentials.

One optional repository **variable** is also supported:

| Variable | Default | Purpose |
|----------|---------|---------|
| `DOCKERHUB_IMAGE` | `<DOCKERHUB_USERNAME>/labelize` | Docker Hub repository to push to, if the name differs |

The first GHCR push creates a package that is **private** by default. To make it
publicly pullable, open the package page → *Package settings* → *Change
visibility* → *Public*.

## Triggers and tags

| Event | Tags produced |
|-------|---------------|
| Push to `main` | `main`, `edge` |
| Push of tag `vX.Y.Z` | `X.Y.Z`, `X.Y`, `latest` |
| Push of tag `vX.Y.Z-rc.N` | `X.Y.Z-rc.N` only — prereleases never move `latest` |
| Push of tag matching `v[0-9]*` but not semver (`v1.3`, `v1abc`) | nothing — the run starts, then skips publishing with a warning |
| Push of any other tag (`vtest`, `nightly`) | nothing — the workflow does not start at all |
| Pull request | none — builds and smoke-tests only, no publish |

The tag scheme follows mainstream OSS practice (e.g. Grafana, n8n): `latest`
plus full and minor semver tags on a release, and a single moving `edge` tag on
the default branch. There is deliberately **no per-commit `sha-<full>` tag**
(each `main` push would otherwise leave a permanent, ever-growing tag on Docker
Hub) and no **bare `X` major-only tag**.

`latest` is guarded in two layers, because a tag that produces no version tags at
all must never be able to repoint it at an arbitrary commit:

1. **The trigger glob** is `tags: ['v[0-9]*']`, not `v*`. A tag that does not begin
   with `v` followed by a digit — `vtest`, `vlatest`, `nightly` — never starts a
   run, so it costs nothing and can affect nothing.
2. **The `prepare` job** then matches the tag against `^v[0-9]+\.[0-9]+\.[0-9]+$`
   for `latest`, and against the same pattern plus an optional
   prerelease/build suffix to decide whether to publish at all. This catches the
   shapes the glob still lets through — `v1.3`, `v1.3.0.4`, `v1abc` — which would
   otherwise satisfy a naive "is a tag and has no `-` in it" check while
   `metadata-action` emitted no version tags for them.

`latest` is therefore assigned explicitly rather than via `flavor: latest=auto`.

## How the build is arranged

Rust release builds under QEMU emulation are extremely slow, so each architecture
is built on a native runner instead:

```
prepare ──┬─► build (linux/amd64  on ubuntu-latest)   ──┐
          └─► build (linux/arm64  on ubuntu-24.04-arm) ──┴─► merge ─► verify
```

* **prepare** — resolves the target registries once. Secrets cannot be read from
  a job-level `if:`, so the Docker Hub decision is made here and passed down as
  job outputs.
* **build** — builds a single platform and pushes it *by digest* (no tag). Layer
  caching uses the GitHub Actions cache, scoped per platform. Attestations are
  disabled because `push-by-digest` cannot carry attestation manifests.
* **merge** — downloads the per-platform digests and assembles one manifest list
  per registry with `docker buildx imagetools create`. Each registry's manifest
  is built from its own copies of the digests, so no cross-registry blob copying
  is needed.
* **verify** — pulls the published manifest back and checks that the image runs:
  `labelize --version` plus a live `GET /health`.

BuildKit's default of 4 parallel build steps exhausts the 7 GB GitHub runner
during Rust compiles ([moby/buildkit#3969](https://github.com/moby/buildkit/issues/3969)),
so `max-parallelism` is pinned to 2.

Publishing runs are never cancelled by `concurrency` — only pull request builds
are superseded. Aborting between the digest pushes and the manifest merge would
leave dangling untagged digests and no manifest list.

## Build provenance

The `merge` job attaches a signed [build provenance attestation](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations)
to the published GHCR manifest — a record of which repository, commit, and
workflow run produced that exact digest, signed via Sigstore and stored as an OCI
referrer alongside the image.

Anyone can verify an image is genuine before deploying it:

```bash
gh attestation verify oci://ghcr.io/goodboy008/labelize:latest --owner GOODBOY008
```

The exact command, with the digest filled in, is printed in each run's job
summary. Docker Hub is deliberately left out — its OCI referrers support is less
settled than GHCR's.

On pull requests the build job loads the image locally instead of pushing, then
smoke-tests it — `--version`, `GET /health`, and a real `POST /convert` that must
return a non-empty PNG.

PR builds are limited to changes that can actually affect the image (`Dockerfile`,
`.dockerignore`, `docker-compose.yaml`, `Cargo.toml`, `Cargo.lock`, and the
workflow itself). Ordinary source changes are already covered by the `CI`
workflow, so they don't pay for a container rebuild.
