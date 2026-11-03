#!/bin/sh
set -eu
cd "$(dirname "$0")"
VERSION=8.9
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
if command -v curl >/dev/null 2>&1; then curl -fsSL "https://services.gradle.org/distributions/gradle-$VERSION-wrapper.jar" -o "$TMP/gradle-wrapper.jar"; else wget -q "https://services.gradle.org/distributions/gradle-$VERSION-wrapper.jar" -O "$TMP/gradle-wrapper.jar"; fi
mkdir -p gradle/wrapper
cp "$TMP/gradle-wrapper.jar" gradle/wrapper/gradle-wrapper.jar
