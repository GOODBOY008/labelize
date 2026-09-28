# labelize-android

Android support for labelize: parse ZPL/EPL and render labels to PNG/PDF
inside any Android app. This directory contains two things:

| Piece | What it is |
|-------|------------|
| `Cargo.toml` + `src/` | `labelize-android` — a JNI binding crate that mirrors the wasm surface (`wasm/src/lib.rs`) |
| `lib/` | A Gradle Android library module that packages the Kotlin API (`com.goodboy008.labelize`) plus the compiled `.so` files into a single AAR |
| `demo/` | A minimal demo app used for end-to-end verification on an emulator |

Output is bit-identical to the desktop/CLI builds — the same Rust engine runs
on the phone.

## Quick start (consume)

Grab `labelize-android-aar.zip` from the
[latest GitHub release](https://github.com/GOODBOY008/labelize/releases), unzip
into your app module, then:

```kotlin
// app/build.gradle.kts
repositories {
    flatDir { dirs("libs") }
}
dependencies {
    implementation(name = "labelize-android-release", ext = "aar")
}
```

```kotlin
val png = Labelize.renderZplToPng(zpl.toByteArray(), widthMm = 102.0, heightMm = 152.0)
```

See the root README's *Use from Android* section for the full API.

## Build from source

Requirements:

- Rust toolchain with the Android std libraries:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi \
      i686-linux-android x86_64-linux-android
  ```
- Android SDK + NDK r25+ (`$ANDROID_HOME/ndk/...`), JDK 17.

Then:

```bash
cd android
./build.sh                        # cross-compiles .so for all 4 ABIs into lib/src/main/jniLibs/
./gradlew :lib:assembleRelease    # → lib/build/outputs/aar/lib-release.aar
```

`./gradlew :demo:installDebug` installs the demo app, which renders a sample
ZPL label on device and saves `render.png` / `render.pdf` to the app's private
files directory — handy for `adb pull` byte-comparison against a desktop render.

## API surface

Kotlin (`com.goodboy008.labelize.Labelize`):

- `render(src, widthMm, heightMm, dpmm, antialias, pdf, epl): ByteArray` — raw
  entry point, same parameter order as the JS `lz_render` in
  `@goodboy008/labelize-wasm`
- `renderZplToPng` / `renderEplToPng` / `renderZplToPdf` / `renderEplToPdf` —
  conveniences with sensible 4×6" @ 8 dpmm defaults
- `version(): String` — engine/binding version for support diagnostics

Failures throw `LabelizeException(stage, message)` where `stage` is
`STAGE_PARSE` (bad label data, the HTTP-400 analog) or `STAGE_RENDER`
(engine error, the HTTP-500 analog) — the same `1:`/`2:` codes the wasm
binding uses.

## Notes

- The AAR version tracks the engine version in the root `Cargo.toml`
  (`lib/build.gradle.kts`); the Rust binding crate itself stays `0.1.0`,
  like `wasm/`.
- `lib/src/main/jniLibs/` and `target/` are build outputs (gitignored) — CI
  rebuilds them on every run and attaches `labelize-android-aar.zip` to
  releases.
