#!/bin/bash

# Exit on any error
set -e

# Target definitions
TARGET_LINUX_64="x86_64-unknown-linux-gnu"
TARGET_LINUX_32="i686-unknown-linux-gnu"
TARGET_WIN_64="x86_64-pc-windows-gnu"
TARGET_WIN_32="i686-pc-windows-gnu"

# Output directory for the final images/binaries
OUTPUT_DIR="dist"

echo "Setting up build environment..."
mkdir -p "$OUTPUT_DIR"

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    echo "Error: rustup is not installed. Please install it from https://rustup.rs/"
    exit 1
fi

# Function to add target and build
build_target() {
    local target=$1
    local name=$2
    local extension=$3

    echo "========================================================="
    echo "Building for $name ($target)"
    echo "========================================================="
    
    # Ensure the rust target is installed
    rustup target add "$target"

    # Note: Since roughnote uses GTK4, you will need the corresponding C libraries
    # and toolchains installed for cross-compilation (e.g., mingw-w64 for Windows,
    # gcc-multilib and 32-bit GTK libraries for Linux x86_32).
    # If 'cargo' fails due to missing C libraries, consider installing 'cross'
    # (cargo install cross) and using it instead of 'cargo'.
    
    # We will use the CROSS_CMD variable to allow easy swapping between cargo and cross
    BUILD_CMD=${CROSS_CMD:-cargo}

    if $BUILD_CMD build --release --target "$target"; then
        echo "✅ Build successful for $name."
        cp "target/$target/release/roughnote$extension" "$OUTPUT_DIR/roughnote-$name$extension"
        echo "📦 Artifact saved to $OUTPUT_DIR/roughnote-$name$extension"
    else
        echo "❌ Build failed for $name."
        echo "Please ensure you have the required C toolchains and GTK4 development headers for $target."
    fi
    echo ""
}

# Set CROSS_CMD="cross" if you want to use cross-rs for docker-based cross-compilation
# export CROSS_CMD="cross"

# 1. Linux x86_64 (64-bit)
build_target "$TARGET_LINUX_64" "linux-x86_64" ""

# 2. Linux x86_32 (32-bit)
build_target "$TARGET_LINUX_32" "linux-x86_32" ""

# 3. Windows x86_64 (64-bit)
build_target "$TARGET_WIN_64" "windows-x86_64" ".exe"

# 4. Windows x86_32 (32-bit)
build_target "$TARGET_WIN_32" "windows-x86_32" ".exe"

echo "========================================================="
echo "Build process finished."
echo "Check the '$OUTPUT_DIR' directory for the compiled binaries."
