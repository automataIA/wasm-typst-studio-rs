#!/bin/bash
# Size Report Script
# Tracks and reports bundle sizes for web and desktop builds

set -e

echo "📊 ===== SIZE REPORT ====="
echo ""
echo "Generated: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# Function to format bytes to human-readable
format_size() {
    local size=$1
    if [ $size -lt 1024 ]; then
        echo "${size} B"
    elif [ $size -lt 1048576 ]; then
        echo "$(echo "scale=2; $size / 1024" | bc) KB"
    else
        echo "$(echo "scale=2; $size / 1048576" | bc) MB"
    fi
}

# Check if brotli is available
BROTLI_AVAILABLE=false
if command -v brotli &> /dev/null; then
    BROTLI_AVAILABLE=true
fi

# ===== WEB (WASM) =====
echo "🌐 Web Bundle (WASM):"
echo "─────────────────────────────────────"

if [ -d "dist" ]; then
    WASM_FILES=$(find dist -name "*.wasm" 2>/dev/null || true)

    if [ -n "$WASM_FILES" ]; then
        for wasm_file in $WASM_FILES; do
            filename=$(basename "$wasm_file")
            size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
            size_formatted=$(format_size $size)

            echo "  📦 $filename"
            echo "     Uncompressed: $size_formatted"

            # Show gzip estimate
            if command -v gzip &> /dev/null; then
                gzip_size=$(gzip -c "$wasm_file" | wc -c)
                gzip_formatted=$(format_size $gzip_size)
                gzip_reduction=$(echo "scale=1; (1 - $gzip_size / $size) * 100" | bc)
                echo "     Gzip:         $gzip_formatted (${gzip_reduction}% reduction)"
            fi

            # Show brotli size if available
            if [ "$BROTLI_AVAILABLE" = true ]; then
                if [ -f "${wasm_file}.br" ]; then
                    br_size=$(stat -f%z "${wasm_file}.br" 2>/dev/null || stat -c%s "${wasm_file}.br")
                    br_formatted=$(format_size $br_size)
                    br_reduction=$(echo "scale=1; (1 - $br_size / $size) * 100" | bc)
                    echo "     Brotli:       $br_formatted (${br_reduction}% reduction)"
                else
                    br_size=$(brotli -c -9 "$wasm_file" | wc -c)
                    br_formatted=$(format_size $br_size)
                    br_reduction=$(echo "scale=1; (1 - $br_size / $size) * 100" | bc)
                    echo "     Brotli (est): $br_formatted (${br_reduction}% reduction)"
                fi
            fi
            echo ""
        done
    else
        echo "  ⚠️  No WASM files found"
        echo "     Run: trunk build --release"
        echo ""
    fi

    # Show total dist size
    total_size=$(du -sh dist | cut -f1)
    echo "  📁 Total dist/ size: $total_size"
else
    echo "  ⚠️  dist/ directory not found"
    echo "     Run: trunk build --release"
fi

echo ""

# ===== DESKTOP (TAURI) =====
echo "🖥️  Desktop Bundles (Tauri):"
echo "─────────────────────────────────────"

if [ -d "src-tauri/target/release" ]; then
    found_bundles=false

    # Linux AppImage
    if [ -d "src-tauri/target/release/bundle/appimage" ]; then
        echo "  🐧 Linux AppImage:"
        find src-tauri/target/release/bundle/appimage -name "*.AppImage" 2>/dev/null | while read -r f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_formatted=$(format_size $size)
            echo "     $(basename "$f"): $size_formatted"
            found_bundles=true
        done
    fi

    # Linux DEB
    if [ -d "src-tauri/target/release/bundle/deb" ]; then
        echo "  🐧 Linux DEB:"
        find src-tauri/target/release/bundle/deb -name "*.deb" 2>/dev/null | while read -r f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_formatted=$(format_size $size)
            echo "     $(basename "$f"): $size_formatted"
            found_bundles=true
        done
    fi

    # Windows NSIS
    if [ -d "src-tauri/target/release/bundle/nsis" ]; then
        echo "  🪟 Windows NSIS:"
        find src-tauri/target/release/bundle/nsis -name "*.exe" 2>/dev/null | while read -r f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_formatted=$(format_size $size)
            echo "     $(basename "$f"): $size_formatted"
            found_bundles=true
        done
    fi

    # Windows MSI
    if [ -d "src-tauri/target/release/bundle/msi" ]; then
        echo "  🪟 Windows MSI:"
        find src-tauri/target/release/bundle/msi -name "*.msi" 2>/dev/null | while read -r f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_formatted=$(format_size $size)
            echo "     $(basename "$f"): $size_formatted"
            found_bundles=true
        done
    fi

    # macOS DMG
    if [ -d "src-tauri/target/release/bundle/dmg" ]; then
        echo "  🍎 macOS DMG:"
        find src-tauri/target/release/bundle/dmg -name "*.dmg" 2>/dev/null | while read -r f; do
            size=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
            size_formatted=$(format_size $size)
            echo "     $(basename "$f"): $size_formatted"
            found_bundles=true
        done
    fi

    # macOS APP
    if [ -d "src-tauri/target/release/bundle/macos" ]; then
        echo "  🍎 macOS APP:"
        find src-tauri/target/release/bundle/macos -name "*.app" -type d 2>/dev/null | while read -r f; do
            size=$(du -sh "$f" | cut -f1)
            echo "     $(basename "$f"): $size"
            found_bundles=true
        done
    fi

    if [ "$found_bundles" = false ]; then
        echo "  ⚠️  No desktop bundles found"
        echo "     Run: cargo tauri build"
    fi
else
    echo "  ⚠️  Desktop builds not found"
    echo "     Run: cargo tauri build"
fi

echo ""
echo "─────────────────────────────────────"
echo "💡 Tips:"
echo "  • Run 'npm run build:optimized' to build web with compression"
echo "  • Run 'cargo tauri build' to create desktop installers"
if [ "$BROTLI_AVAILABLE" = false ]; then
    echo "  • Install brotli for better compression (sudo apt install brotli)"
fi
echo ""
