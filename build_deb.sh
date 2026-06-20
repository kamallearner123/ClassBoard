#!/bin/bash

# Exit on any error
set -e

echo "========================================================="
echo "Building RoughNote Debian Package"
echo "========================================================="

# 1. Build the release binary
echo "Compiling Rust binary in release mode..."
cargo build --release

# 2. Setup staging area
echo "Setting up package staging area..."
rm -rf target/debian/build
mkdir -p target/debian/build
cp -a debian-pkg/* target/debian/build/

mkdir -p target/debian/build/usr/bin
mkdir -p target/debian/build/usr/share/applications
mkdir -p target/debian/build/usr/share/icons/hicolor/256x256/apps

# 3. Copy files to staging area
echo "Copying binary and assets..."
cp target/release/roughnote target/debian/build/usr/bin/
chmod 755 target/debian/build/usr/bin/roughnote

cp assets/roughnote.desktop target/debian/build/usr/share/applications/
cp assets/logo.png target/debian/build/usr/share/icons/hicolor/256x256/apps/roughnote.png
chmod 644 target/debian/build/usr/share/applications/roughnote.desktop
chmod 644 target/debian/build/usr/share/icons/hicolor/256x256/apps/roughnote.png

# 4. Extract dependencies automatically
echo "Extracting shared library dependencies..."
mkdir -p debian
cat << 'EOF' > debian/control
Source: roughnote
Section: utils
Priority: optional
Maintainer: admin@roughnote.com

Package: roughnote
Architecture: amd64
Description: dummy
EOF
dpkg-shlibdeps target/debian/build/usr/bin/roughnote -O > debian/substvars 2>/dev/null || true
DEPENDS=$(grep -E '^shlibs:Depends=' debian/substvars | cut -d= -f2- || true)
rm -rf debian

if [ -n "$DEPENDS" ]; then
    echo "Found dependencies: $DEPENDS"
    sed -i "s/^Depends:.*/Depends: ${DEPENDS}/" target/debian/build/DEBIAN/control
else
    echo "Warning: Could not automatically determine dependencies. Using defaults."
fi

# 5. Extract metadata for naming
VERSION=$(grep -E '^Version:' target/debian/build/DEBIAN/control | awk '{print $2}')
ARCH=$(grep -E '^Architecture:' target/debian/build/DEBIAN/control | awk '{print $2}')
PACKAGE_NAME=$(grep -E '^Package:' target/debian/build/DEBIAN/control | awk '{print $2}')
DEB_FILENAME="${PACKAGE_NAME}_${VERSION}-1_${ARCH}.deb"

# 6. Build the .deb file
echo "Building the .deb file..."
dpkg-deb --build target/debian/build "target/debian/$DEB_FILENAME"

# Cleanup staging area
rm -rf target/debian/build

echo "========================================================="
echo "✅ Debian package successfully created at: target/debian/$DEB_FILENAME"
echo "========================================================="
