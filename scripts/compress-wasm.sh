#!/bin/bash
# WASM Compression Script
# 1. Optionally optimizes WASM with wasm-opt (if available)
# 2. Compresses WASM files with brotli for optimal web delivery

set -e

echo "🗜️  WASM Optimization & Compression"
echo ""

DIST_DIR="dist"
WASM_OPT_AVAILABLE=false

# Check if wasm-opt is installed
if command -v wasm-opt &> /dev/null; then
    WASM_OPT_AVAILABLE=true
    echo "✅ wasm-opt found - will optimize WASM files"
else
    echo "⚠️  wasm-opt not found - skipping optimization"
    echo "   Install: npm install -g wasm-opt or from binaryen package"
fi

# Check if brotli is installed
if ! command -v brotli &> /dev/null; then
    echo "❌ brotli is not installed!"
    echo "Install with: sudo apt-get install brotli (Ubuntu/Debian)"
    echo "           or: brew install brotli (macOS)"
    exit 1
fi

echo ""

# Find and compress all WASM files
WASM_FILES=$(find "$DIST_DIR" -name "*.wasm" 2>/dev/null || true)

if [ -z "$WASM_FILES" ]; then
    echo "⚠️  No WASM files found in $DIST_DIR/"
    echo "   Run 'trunk build --release' first"
    exit 1
fi

for wasm_file in $WASM_FILES; do
    original_size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
    original_mb=$(echo "scale=2; $original_size / 1024 / 1024" | bc)

    echo "📦 Processing: $(basename "$wasm_file")"
    echo "   Original: ${original_mb} MB"

    # Step 1: Optimize with wasm-opt if available
    if [ "$WASM_OPT_AVAILABLE" = true ]; then
        echo "   🔧 Optimizing with wasm-opt..."
        wasm_opt_output="${wasm_file}.opt"
        # Enable all required WebAssembly features
        wasm-opt --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int -Oz "$wasm_file" -o "$wasm_opt_output" 2>/dev/null || {
            echo "   ⚠️  wasm-opt failed, using original WASM"
            wasm_opt_output="$wasm_file"
        }

        if [ -f "$wasm_opt_output" ] && [ "$wasm_opt_output" != "$wasm_file" ]; then
            opt_size=$(stat -f%z "$wasm_opt_output" 2>/dev/null || stat -c%s "$wasm_opt_output")
            opt_mb=$(echo "scale=2; $opt_size / 1024 / 1024" | bc)
            opt_reduction=$(echo "scale=1; (1 - $opt_size / $original_size) * 100" | bc)
            echo "   Optimized: ${opt_mb} MB (${opt_reduction}% reduction)"

            # Replace original with optimized
            mv "$wasm_opt_output" "$wasm_file"
            original_size=$opt_size
        fi
    fi

    # Step 2: Compress with brotli (level 9 = maximum compression)
    echo "   🗜️  Compressing with brotli..."
    brotli -9 -f "$wasm_file" -o "${wasm_file}.br"

    compressed_size=$(stat -f%z "${wasm_file}.br" 2>/dev/null || stat -c%s "${wasm_file}.br")
    compressed_mb=$(echo "scale=2; $compressed_size / 1024 / 1024" | bc)
    reduction=$(echo "scale=1; (1 - $compressed_size / $original_size) * 100" | bc)

    echo "   Brotli: ${compressed_mb} MB (${reduction}% reduction from optimized)"
    echo ""
done

echo "✅ Optimization & Compression complete!"
echo ""
echo "📋 Next steps for deployment:"
echo "   1. Upload both .wasm and .wasm.br files to your server"
echo "   2. Configure server to serve .wasm.br with:"
echo "      Content-Encoding: br"
echo "      Content-Type: application/wasm"
echo ""
echo "   For GitHub Pages, add _headers file:"
echo "   /*.wasm.br"
echo "     Content-Encoding: br"
echo "     Content-Type: application/wasm"
echo ""
if [ "$WASM_OPT_AVAILABLE" = false ]; then
    echo "💡 Tip: Install wasm-opt for additional 5-10% size reduction:"
    echo "   npm install -g wasm-opt"
    echo "   or: sudo apt-get install binaryen"
fi
