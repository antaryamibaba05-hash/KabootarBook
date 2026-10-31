# Upload KabootarBook from Termux

The build is run by GitHub Actions, not in Termux. This folder contains source and the Actions APK workflow.

1. Install Git in Termux:

```sh
pkg update
pkg install git
```

2. Extract `KabootarBook-starter.zip` into a folder you can access. If you saved it under Android Downloads, grant Termux storage access once with `termux-setup-storage`, then:

```sh
mkdir -p ~/projects
cd ~/projects
unzip /sdcard/Download/KabootarBook-starter.zip
cd KabootarBook
```

3. Create an empty repository on GitHub, then set your author identity and push (replace the remote URL):

```sh
git init
git config user.name "Rana Adeel Jameel"
git config user.email "YOUR_GITHUB_EMAIL"
git add .
git commit -m "Start KabootarBook"
git branch -M main
git remote add origin git@github.com:YOUR_GITHUB_NAME/YOUR_REPOSITORY.git
git push -u origin main
```

Authenticate with GitHub's browser/device flow or an SSH key. Never put a token in source files or commit it. If this folder is already a Git checkout, don't run `git init` again.

4. On GitHub, open **Actions → Android APK**, wait for the run, then download the `kabootarbook-android-apk` artifact. The APK is a testing/debug build. Android may ask permission to install from the browser or file manager.

The GitHub Actions APK build is a first draft and still needs an actual successful CI run. If it fails, open the run's failed step and share its error log for diagnosis.
