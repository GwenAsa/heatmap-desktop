# Heatmap on Android

The Android app is the same React UI and the same Rust core (SQLite), packaged with Tauri 2. It stores its data on the phone, offline.

> **Status: experimental.** The Android build is produced by CI and has not been run on a physical phone by the author yet. Please report anything odd.

## What works
- Trackers (all types), daily logging, notes, explicit zeros
- Month and year heatmaps with every day numbered; cells are sized to your screen
- Goals, weekly goals, freeze days, streaks, statistics, insights, correlation
- Quick add line (`coding 90m yesterday`)
- Studio palettes and shapes, light/dark theme

## Not available on Android (yet)
| Feature | Why |
|---|---|
| Desktop widgets, tray, global shortcut | Desktop-only concepts |
| Home-screen widgets | Need a native Android App Widget (Kotlin); a different piece of work from this app |
| Git tracker | No `git` binary on phones |
| JSON/CSV import, export, database backup, SVG/PNG export | Android's scoped storage returns `content://` URIs; the file plugin integration is not done yet |

Because import/export is not available yet, the phone and desktop databases are independent for now.

## Get the APK (no Android tools needed)
1. On GitHub open **Actions → Android APK → Run workflow**.
2. When it finishes (about 15–30 minutes the first time), open the run and download the **heatmap-android-apk** artifact (a zip containing `Heatmap-android-debug.apk`).
3. Copy the APK to the phone, open it and allow *Install unknown apps* for your file manager when Android asks.

Pushing a `v*` tag also attaches the APK to the Release page.

The APK is **debug-signed**, which is fine for installing and testing. A properly signed release build (and an AAB for the Play Store) needs your own keystore.

## Build it yourself (Zorin OS / Ubuntu)
```bash
sudo apt install openjdk-17-jdk
# Android SDK + NDK: install Android Studio, then SDK Manager → "NDK (Side by side)" and "Android SDK Platform 34"
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/<installed-version>"
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

npm ci
npx tauri android init
npx tauri android build --debug --apk --target aarch64
# APK: src-tauri/gen/android/app/build/outputs/apk/
```
Use `rustup` (not Ubuntu's `rustc` package) so the Android targets can be installed.

## Roadmap for mobile
1. Import/export through the Tauri file plugin (read and write `content://` URIs)
2. A native Android home-screen widget
3. Release signing and AAB for the Play Store
