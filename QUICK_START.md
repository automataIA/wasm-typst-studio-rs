# 🚀 Quick Start Guide - Typst Studio

## Installation

### Prerequisites

```bash
# Install Rust nightly
rustup toolchain install nightly --allow-downgrade

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Trunk (for web)
cargo install trunk

# Install Tauri CLI (for desktop)
cargo install tauri-cli

# Install Node dependencies
npm install
```

## Development

### Option 1: Web App

```bash
# Build CSS (first time only)
npm run build-css

# Start web dev server
npm run dev
# or
trunk serve --open

# Access at: http://localhost:1420
```

### Option 2: Desktop App

```bash
# Start desktop app with hot-reload
npm run tauri:dev
# or
cargo tauri dev

# Automatic: trunk serve starts, desktop window opens
```

## Production Build

### Web Build

```bash
# Standard build
npm run build
# or
trunk build --release

# Output: dist/
```

### Desktop Build

```bash
# Build installer for your OS
npm run tauri:build
# or
cargo tauri build

# Output: src-tauri/target/release/bundle/
# Linux: .deb, .AppImage, .rpm
# Windows: .msi, .exe
# macOS: .dmg, .app
```

## Common Commands

| Task | Command |
|------|---------|
| Web dev server | `trunk serve` or `npm run dev` |
| Desktop dev | `cargo tauri dev` or `npm run tauri:dev` |
| Web production build | `trunk build --release` or `npm run build` |
| Desktop production build | `cargo tauri build` or `npm run tauri:build` |
| Build CSS | `npm run build-css` |
| Check Rust code | `cargo check` |
| Format code | `cargo fmt` |

## Project Scripts (package.json)

```json
{
  "dev": "trunk serve",                  // Web dev server
  "build": "trunk build --release",      // Web production
  "build-css": "tailwindcss ...",        // Compile Tailwind
  "tauri:dev": "cargo tauri dev",        // Desktop dev
  "tauri:build": "cargo tauri build"     // Desktop production
}
```

## Troubleshooting

### Port Already in Use

```bash
# Kill existing trunk process
pkill trunk

# Or change port in Trunk.toml:
# port = 1421
```

### CSS Not Updating

```bash
# Rebuild CSS
npm run build-css

# Clear trunk cache
rm -rf .trunk/
```

### Desktop Window Not Opening (WSL)

```bash
# WSL2 has limited GUI support
# Test on native Linux/Windows/macOS instead

# Verify backend compiles:
cd src-tauri && cargo check
```

### WASM Build Fails

```bash
# Clean build
trunk clean
cargo clean

# Rebuild
trunk build
```

## File Structure Quick Reference

```
src/           → Frontend (Leptos) - shared by web & desktop
src-tauri/     → Backend (Tauri) - desktop only
dist/          → Web build output
public/        → Static assets
examples/      → Example .typ files
tailwind.css   → Tailwind source
```

## Port Configuration

- **Development**: http://localhost:1420 (Trunk.toml)
- **Production Web**: Configurable via Trunk-release.toml
- **Desktop**: Uses trunk dev server internally

## Next Steps

1. **Try the web app**: `trunk serve --open`
2. **Try desktop app**: `cargo tauri dev`
3. **Customize**: Edit `src/lib.rs` for app logic
4. **Deploy web**: `trunk build --config Trunk-release.toml`
5. **Distribute desktop**: `cargo tauri build`

## Documentation Links

- **Main README**: [README.md](./README.md) - Full documentation
- **Tauri Integration**: [TAURI_INTEGRATION.md](./TAURI_INTEGRATION.md) - Desktop details
- **Typst Docs**: https://typst.app/docs
- **Leptos Book**: https://book.leptos.dev/
- **Tauri Docs**: https://v2.tauri.app/

---

**Need help?** Check the full [README.md](./README.md) or [TAURI_INTEGRATION.md](./TAURI_INTEGRATION.md)
