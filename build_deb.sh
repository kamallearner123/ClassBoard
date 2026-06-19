#!/bin/bash

# Exit on any error
set -e

echo "========================================================="
echo "Building RoughNote Debian Package"
echo "========================================================="

# 1. Build the release binary
echo "Compiling Rust binary in release mode..."
cargo build --release

# 2. Ensure the package directory structure exists
echo "Setting up package structure..."
mkdir -p debian-pkg/usr/bin
mkdir -p debian-pkg/usr/share/applications
mkdir -p debian-pkg/usr/share/icons/hicolor/256x256/apps

# 3. Copy the built binary into the debian package structure
echo "Copying binary to debian-pkg/usr/bin..."
cp target/release/roughnote debian-pkg/usr/bin/
chmod 755 debian-pkg/usr/bin/roughnote

# 3.5 Copy desktop file and icon
echo "Copying desktop entry and icon..."
cp assets/roughnote.desktop debian-pkg/usr/share/applications/
cp assets/logo.png debian-pkg/usr/share/icons/hicolor/256x256/apps/roughnote.png
chmod 644 debian-pkg/usr/share/applications/roughnote.desktop
chmod 644 debian-pkg/usr/share/icons/hicolor/256x256/apps/roughnote.png


# 4. Extract version and architecture from DEBIAN/control
VERSION=$(grep -E '^Version:' debian-pkg/DEBIAN/control | awk '{print $2}')
ARCH=$(grep -E '^Architecture:' debian-pkg/DEBIAN/control | awk '{print $2}')
PACKAGE_NAME=$(grep -E '^Package:' debian-pkg/DEBIAN/control | awk '{print $2}')

# The debian package naming convention is <package>_<version>-<revision>_<architecture>.deb
# We will assume revision 1 for local builds.
DEB_FILENAME="${PACKAGE_NAME}_${VERSION}-1_${ARCH}.deb"

# 5. Create the target directory for the debian package
mkdir -p target/debian

# 6. Build the .deb file
echo "Building the .deb file..."
dpkg-deb --build debian-pkg "target/debian/$DEB_FILENAME"

echo "========================================================="
echo "✅ Debian package successfully created at: target/debian/$DEB_FILENAME"
echo "========================================================="
