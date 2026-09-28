#!/usr/bin/env bash
# Cross-compiles the labelize JNI library for the Android ABIs this package
# ships, then drops each liblabelize_android.so into the Gradle library's
# jniLibs so `./gradlew :lib:assembleRelease` packages them into the AAR.
#
# Requirements:
#   - Rust targets: rustup target add aarch64-linux-android \
#         armv7-linux-androideabi i686-linux-android x86_64-linux-android
#   - Android NDK r25+ under $ANDROID_HOME/ndk (any install works; the newest
#     version directory is picked automatically)
set -euo pipefail

cd "$(dirname "$0")"

ANDROID_API="${ANDROID_API:-24}"
JNI_LIBS="lib/src/main/jniLibs"
LIB_NAME="liblabelize_android.so"

# abi | rust target | NDK clang triplet prefix
ABIS=(
    "arm64-v8a|aarch64-linux-android|aarch64-linux-android"
    "armeabi-v7a|armv7-linux-androideabi|armv7a-linux-androideabi"
    "x86|i686-linux-android|i686-linux-android"
    "x86_64|x86_64-linux-android|x86_64-linux-android"
)

# ANDROID_ABIS="arm64-v8a,x86_64" builds a subset (CI uses this to save time).
if [[ -n "${ANDROID_ABIS:-}" ]]; then
    IFS=',' read -r -a wanted <<<"$ANDROID_ABIS"
    filtered=()
    for entry in "${ABIS[@]}"; do
        for w in "${wanted[@]}"; do
            [[ "$entry" == "$w|"* ]] && filtered+=("$entry") && break
        done
    done
    ABIS=("${filtered[@]}")
fi

if [[ -z "${ANDROID_HOME:-}" ]]; then
    for candidate in "$HOME/Library/Android/sdk" "$HOME/Android/Sdk"; do
        if [[ -d "$candidate/ndk" ]]; then ANDROID_HOME="$candidate"; break; fi
    done
fi
[[ -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME/ndk" ]] || {
    echo "error: no NDK found under \$ANDROID_HOME/ndk (set ANDROID_HOME)" >&2
    exit 1
}

# Newest NDK wins (plain sort -V over version-numbered directories).
NDK_HOME="$ANDROID_HOME/ndk/$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
NDK_BIN="$NDK_HOME/toolchains/llvm/prebuilt/$(ls "$NDK_HOME/toolchains/llvm/prebuilt" | head -1)/bin"
echo "NDK: $NDK_HOME"

for entry in "${ABIS[@]}"; do
    IFS='|' read -r abi target triplet <<<"$entry"
    export "CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER=$NDK_BIN/${triplet}${ANDROID_API}-clang"
    echo "==> cargo build --release --target $target"
    cargo build --release --target "$target" --target-dir target
    out="$JNI_LIBS/$abi"
    mkdir -p "$out"
    cp "target/$target/release/$LIB_NAME" "$out/"
    echo "    -> $out/$LIB_NAME ($(du -h "target/$target/release/$LIB_NAME" | cut -f1))"
done

echo "jniLibs ready — now run: ./gradlew :lib:assembleRelease"
