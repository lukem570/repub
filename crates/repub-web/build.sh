#!/bin/bash
# Build script for Repub Web

set -e

echo "Building Repub Web..."

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Build WASM
echo "Building WASM module..."
wasm-pack build --target web --release

# Copy static files to pkg
echo "Copying static files..."
cp www/index.html pkg/
cp www/manifest.json pkg/
cp www/favicon.svg pkg/
cp www/sw.js pkg/

# Generate placeholder icons
echo "Generating icons..."
if command -v convert &> /dev/null; then
    # Using ImageMagick if available
    convert -size 192x192 xc:#2563eb -fill white -gravity center \
        -font Arial-Bold -pointsize 120 -annotate 0 "P" pkg/icon-192.png
    convert -size 512x512 xc:#2563eb -fill white -gravity center \
        -font Arial-Bold -pointsize 320 -annotate 0 "P" pkg/icon-512.png
else
    echo "ImageMagick not found, skipping icon generation"
    # Create placeholder icons with a simple SVG-to-PNG approach
fi

echo ""
echo "Build complete! Files are in pkg/"
echo ""
echo "To test locally:"
echo "  cd pkg && python3 -m http.server 8080"
echo "  Then open http://localhost:8080"
