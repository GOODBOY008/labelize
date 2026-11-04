# Android JNI host smoke check

`AndroidSmoke.java` exercises the Kotlin/JNI boundary on a host JVM. It checks
vector PNG dimensions, PDF output, a multipage request and exception stages.

Build `android/Cargo.toml` for the host and assemble the Android library as
described in [android/README.md](../../android/README.md). Compile this source
against the AAR's `classes.jar`, then run it with that jar, the Kotlin standard
library and the host native library directory on `java.library.path`.
Write compiled classes under ignored `output/`, for example:

```sh
# Run from the repository root, with the built artifacts configured below.
# LABELIZE_ANDROID_CLASSPATH contains classes.jar and the Kotlin standard library.
# LABELIZE_ANDROID_NATIVE_DIR contains the host JNI library.
mkdir -p output/android-smoke
javac -cp "$LABELIZE_ANDROID_CLASSPATH" -d output/android-smoke e2e/android/AndroidSmoke.java
java -Djava.library.path="$LABELIZE_ANDROID_NATIVE_DIR" \
  -cp "output/android-smoke:$LABELIZE_ANDROID_CLASSPATH" AndroidSmoke
```

The example uses the macOS/Linux classpath separator (`:`); use `;` on Windows.
This verifies host linkage and rendering, not physical Android-device printing.
