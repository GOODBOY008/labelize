# Tutorial: Android

Render ZPL/EPL to PNG/PDF inside an Android app with the self-contained
`labelize-android` AAR (`com.goodboy008.labelize`). Native libraries for
`arm64-v8a`, `armeabi-v7a`, `x86_64`, and `x86` are bundled; output is
bit-identical to the desktop builds — the same Rust engine runs on the phone.

## 1. Get the AAR

Download `labelize-android-aar.zip` from the
[latest release](https://github.com/GOODBOY008/labelize/releases) and unzip it
into your app module's `libs/` folder.

## 2. Wire it up

```kotlin
// app/build.gradle.kts
repositories {
    flatDir { dirs("libs") }
}
dependencies {
    implementation(name = "labelize-android-release", ext = "aar")
}
```

No external dependencies — fonts are embedded in the native library.

## 3. Render a label (Kotlin)

```kotlin
import com.goodboy008.labelize.Labelize
import com.goodboy008.labelize.LabelizeException

val zpl = "^XA^FO50,50^A0N,40,40^FDHello Android^FS^XZ".toByteArray()

val png: ByteArray = try {
    Labelize.renderZplToPng(zpl, widthMm = 102.0, heightMm = 152.0)
} catch (e: LabelizeException) {
    // e.stage == LabelizeException.STAGE_PARSE  → bad label data
    // e.stage == LabelizeException.STAGE_RENDER → engine error
    return
}

// save / display — e.g. openFileOutput("label.png", MODE_PRIVATE).use {
//   it.write(png) }
```

Java calls the same static methods.

## 4. API surface

`Labelize` (Kotlin/Java):

| Method | Purpose |
|--------|---------|
| `renderZplToPng(src, widthMm, heightMm, …)` | ZPL → PNG |
| `renderZplToPdf(src, widthMm, heightMm, …)` | ZPL → PDF |
| `renderEplToPng(src, widthMm, heightMm, …)` | EPL → PNG |
| `renderEplToPdf(src, widthMm, heightMm, …)` | EPL → PDF |
| `render(src, widthMm, heightMm, dpmm, antialias, pdf, epl)` | raw entry point — same parameter order as the JS `lz_render` |
| `version()` | engine/binding version, for support diagnostics |

The convenience methods default to 4″ × 6″ at 8 dpmm; use `render(…)` for full
control (custom dpmm, antialiasing).

Failures throw `LabelizeException(stage, message)`:

- `STAGE_PARSE` — the label data is invalid (the HTTP-400 analog)
- `STAGE_RENDER` — engine error (the HTTP-500 analog)

These mirror the `1:` / `2:` stage codes used by the wasm binding.

## 5. Threading note

`render*` calls run synchronously on the calling thread and take a few
milliseconds per label on modern hardware. On Android, invoke them from a
background thread (e.g. `Dispatchers.Default` in coroutines) and hop back to
the main thread to update UI.

```kotlin
withContext(Dispatchers.Default) {
    Labelize.renderZplToPng(zpl, 102.0, 152.0)
}
```

## 6. Build the AAR from source

Requirements: Rust toolchain with the four Android targets, Android SDK +
NDK r25+, JDK 17.

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi \
    i686-linux-android x86_64-linux-android
cd android
./build.sh                        # cross-compiles .so for all 4 ABIs
./gradlew :lib:assembleRelease    # → lib/build/outputs/aar/lib-release.aar
```

`./gradlew :demo:installDebug` installs a demo app that renders a sample label
on device — handy for `adb pull` byte-comparison against a desktop render.
See [`android/README.md`](../../android/README.md) for details.

## 7. What's next

- [JavaScript/WASM tutorial](javascript-wasm.md) — the same engine for web
- [Rust library tutorial](rust-library.md) — the API beneath the bindings
