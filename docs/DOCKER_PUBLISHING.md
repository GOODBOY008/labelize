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
| Push to `main` | `main`, `edge`, `sha-<full-commit-sha>` |
| Push of tag `vX.Y.Z` | `X.Y.Z`, `X.Y`, `X`, `latest` |
| Push of tag `vX.Y.Z-rc.N` | `X.Y.Z-rc.N` only — prereleases never move `latest` |
| Push of a non-semver `v*` tag | nothing — publish is skipped with a warning |
| Pull request | none — builds and smoke-tests only, no publish |

The `X` major tag is skipped for `v0.*` releases, where a major-only tag would be
misleading.

`latest` is assigned explicitly rather than via `flavor: latest=auto`, and it is
gated on the `prepare` job having matched the tag against `^v[0-9]+\.[0-9]+\.[0-9]+$`.
The workflow triggers on `v*`, which is broader than semver, so a scratch tag such
as `vtest` would otherwise generate no version tags at all while still satisfying a
naive "is a tag and has no `-` in it" check — repointing `latest` at an arbitrary
commit. Such tags are rejected outright instead.

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
