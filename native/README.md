# Native KabootarBook (Compose + Rust)

This directory is the native Android direction: Jetpack Compose renders the screens; Rust owns SQLite records and flight calculations through JNI. It does not embed a browser or run JavaScript in the app UI. Android's system back dismisses an open dialog first and otherwise pops the native screen stack.

## Build

GitHub Actions builds and uploads `app/build/outputs/apk/debug/app-debug.apk` as `kabootarbook-native-android-apk`. The debug APK is signed with Android's debug key. It is for testing/sideloading and is not the Play Store release key. A stable signing key is required for reliable updates between CI builds.

Local build prerequisite: JDK 17, Android SDK platform/build-tools 35, NDK 27.2.12479018, Rust 1.99.1 Android targets, and cargo-ndk 4.1.2. Gradle is 8.9 and Android Gradle Plugin is 8.7.3.

## Important data-migration caution

The native app provides JSON backup import/export, but Android WebView localStorage is not automatically visible to a native Android activity. Do not uninstall the old app or clear its data while testing. Export and retain a full-state JSON backup from the old app first. Import is only complete after counts, linked sessions, custom fields, settings, and any active flight have been checked. The old app's Android SQLite location has not yet been verified against the new private database path.

The Rust database currently holds core pigeon fields, shared completed/active flight data, and settings JSON. The first native UI draft has limited field editing and no native CSV backup UI yet; those must be completed before treating this as a production replacement.
