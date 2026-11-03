# KabootarBook

**An offline loft journal for pigeon keepers.** Made by Rana Adeel Jameel.

KabootarBook records pigeon details and flight sessions. Release and return times are saved as timestamps; duration, totals, averages, and personal bests are derived from those timestamps. It does not measure altitude or track pigeons by GPS.

## Current state

This repository contains a working-in-browser Tauri hybrid prototype, an atomic SQLite snapshot adapter with a Tauri SQL migration, and a GitHub Actions Android APK workflow draft. The browser preview uses local storage; the Android SQL path and APK build have not been run here. This remains an early prototype, not a verified production Android tracker. Back up data and do not rely on it as the only copy of important records.

## Preview locally

Requires Node.js 22 or later.

```sh
npm install
npm run dev
```

Open the local URL shown by Vite. This browser preview is only for UI iteration; it does not emulate Android lifecycle or SQLite storage.

## Termux → GitHub → APK

Termux is used to commit and upload the source. GitHub Actions builds a debug APK on an Ubuntu runner.

1. Install Git in Termux: `pkg update && pkg install git`.
2. Make a repository on GitHub, then in Termux enter this project directory.
3. Set your Git author name/email once, then run:

```sh
git init
git add .
git commit -m "Start KabootarBook"
git branch -M main
git remote add origin git@github.com:YOUR_GITHUB_NAME/YOUR_REPOSITORY.git
git push -u origin main
```

Replace the remote with the HTTPS URL if you use GitHub's browser/device authentication. Never put a personal access token into project files or a command. Authenticate using GitHub's supported browser/device flow or configure an SSH key in Termux and add its public key to GitHub.

4. On GitHub, open **Actions → Android APK**. Pushes to `main` start a build; **Run workflow** can also start one manually.
5. Open a successful run and download the `kabootarbook-android-apk` artifact. Unzip it and install the APK on Android. If prompted, allow your browser/file manager to install unknown apps.

This workflow is intended to create a debug APK for testing/sideloading. A Play Store release requires a release build, signing configuration, and a protected keystore; do not commit signing keys. The workflow and Android build have not yet been verified in this environment.

## Implemented in this prototype

- Responsive modern loft-journal interface with Today, Pigeons, Flights, Insights, and Settings screens.
- Add/edit pigeon records with case-insensitive duplicate ring checks; view history, archive/restore, and delete only birds without flight history.
- Start/stop one persisted flight timer; one shared flight may include multiple pigeons. Manually edit historical release/return timestamps.
- Calculate durations, session counts, totals, averages, longest flight, weekly summaries, and pigeon personal bests from timestamps.
- CSV backup export and validated append-only import; conflicting IDs/ring numbers or unknown linked pigeons cancel the import.
- Saved appearance/date/duration/list-order preferences and configurable extra text fields.
- Browser preview state persists through localStorage. Tauri Android source includes a SQLite migration and one atomic JSON state-row adapter; verify generated Android project and plugin behavior before relying on it.
- Initial GitHub Actions APK workflow draft.

## Remaining work before a dependable release

- Run and fix Tauri Android initialization and the Actions APK build; the APK has not yet been verified.
- Integrate and test native SQLite transaction behavior, migrations, persistence errors, and recovery on Android process death/reboot. The current Android adapter rewrites a snapshot in a transaction and needs device/CI testing.
- Add automated end-to-end and real database tests; current Node tests check key invariants, not Android runtime behavior.
- Improve import/export test coverage, visual QA across Android versions, and release signing (optional).
- Keep describing this honestly as a prototype until these checks pass.
