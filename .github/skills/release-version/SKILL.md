---
name: release-version
description: 'Cut a new labelize release: bump Cargo.toml version, add CHANGELOG section, update Cargo.lock, run tests, commit, push tag, and draft GitHub Release. Use when: releasing a new version, bumping version, cutting a release, publishing to crates.io, updating changelog for release, tagging a release, creating a GitHub Release draft.'
argument-hint: 'New version number (e.g. 0.6.0) and optional release notes summary'
---

# Release a New Labelize Version

## What This Skill Does

Walks through every step required to cut a versioned release of `labelize` — from bumping files through to a pushed tag that triggers the automated GitHub Release, crates.io publish, and Homebrew formula update.

## Files That Must Be Updated

| File | Change |
|------|--------|
| `Cargo.toml` | `version = "X.Y.Z"` |
| `e2e/sdk/Cargo.toml` | `version`, `labelize` dependency (×2) all set to `X.Y.Z` |
| `Cargo.lock` | Auto-regenerated (run `cargo build`) |
| `CHANGELOG.md` | New `## [X.Y.Z] - YYYY-MM-DD` section at the top |

## Procedure

### 1. Determine the New Version

Apply [Semantic Versioning](https://semver.org/):
- **patch** (`0.5.0 → 0.5.1`) — bug fixes only, no new features
- **minor** (`0.5.0 → 0.6.0`) — new features, backwards-compatible
- **major** (`0.5.0 → 1.0.0`) — breaking API changes

Current version is in `Cargo.toml`: `grep '^version' Cargo.toml`

### 2. Update CHANGELOG.md

Add a new section **above** the previous latest release, following the existing format:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- **Feature Name** — Short description

### Fixed
- **Bug Name** — Short description

### Changed
- **Change Name** — Short description
```

Use today's date in `YYYY-MM-DD` format. Collect entries from `git log --oneline <prev-tag>..HEAD`.

### 3. Bump Cargo.toml

Edit `Cargo.toml`, change the `version` field:
```toml
version = "X.Y.Z"
```

### 4. Regenerate Cargo.lock

```bash
cargo build
```

This updates `Cargo.lock` to match the new version. Verify it compiled cleanly.

### 5. Run Tests

```bash
# Unit tests
cargo test --test 'unit_*'

# E2E golden tests
cargo test --test 'e2e_*'

# HTTP e2e shell tests (optional but recommended)
PATH="$PATH:target/debug" bash e2e/http/test_http.sh

# CLI e2e shell tests (optional, requires labelize on PATH)
PATH="$PATH:target/debug" bash e2e/cli/test_cli.sh
```

All tests must pass before tagging. Fix any failures before proceeding.

### 6. Commit

Stage and commit **exactly** these files:

```bash
git add Cargo.toml e2e/sdk/Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
```

### 7. Push the Commit

```bash
git push origin main
```

### 8. Create and Push the Tag

Before the first stable release using trusted publishing, open the npm package
settings for `@goodboy008/labelize-wasm` and add a **GitHub Actions** trusted
publisher. Enter owner `GOODBOY008`, repository `labelize`, and workflow
filename `release.yml`. Leave the environment blank and enable **Allow npm publish**.
The first successful publish must occur within two days of setup.
[npm package settings](https://www.npmjs.com/package/@goodboy008/labelize-wasm/access) · [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/)

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Pushing a tag matching `v*` triggers the `release.yml` workflow automatically:
- Builds binaries for Linux x86_64, macOS aarch64, macOS x86_64
- Creates a **GitHub Release** with auto-generated notes and binary assets
- Publishes the crate to **crates.io**
- Publishes `@goodboy008/labelize-wasm` to npm on stable tags
- Dispatches a Homebrew formula update to `GOODBOY008/homebrew-labelize`

### 9. Verify the GitHub Release (Optional)

After the workflow completes (~5 min), check:
- `https://github.com/GOODBOY008/labelize/releases/tag/vX.Y.Z` has 3 binary assets
- `https://crates.io/crates/labelize` shows the new version
- For a stable tag, `npm view @goodboy008/labelize-wasm@X.Y.Z version dist.attestations.url --registry=https://registry.npmjs.org` shows the version and its attestation URL.
- Homebrew formula update PR opened in `GOODBOY008/homebrew-labelize`

## Quick Checklist

- [ ] Version decided (semver)
- [ ] `CHANGELOG.md` updated with today's date
- [ ] `Cargo.toml` version bumped
- [ ] `cargo build` succeeds → `Cargo.lock` updated
- [ ] `cargo test --test 'unit_*'` passes
- [ ] `cargo test --test 'e2e_*'` passes
- [ ] `git add Cargo.toml e2e/sdk/Cargo.toml Cargo.lock CHANGELOG.md && git commit -m "chore: release vX.Y.Z"`
- [ ] `git push origin main`
- [ ] `git tag vX.Y.Z && git push origin vX.Y.Z`
- [ ] GitHub Actions `release.yml` workflow completes successfully

## Example Commit History Pattern

```
chore: release v0.5.0
feat: add label playground web UI
fix: MaxiCode ECC pipeline
...
chore: release v0.4.0
```
