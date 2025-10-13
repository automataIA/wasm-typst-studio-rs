# Typst Studio WASM

A modern [Typst](https://typst.app) editor built entirely in Rust using WebAssembly. Available as both a **web application** and a **native desktop app** (via Tauri). Write and preview Typst documents with real-time compilation, syntax highlighting, and multi-page support.

## ✨ Features

### Core Features
- **🚀 100% Rust + WASM** - No JavaScript required, fully compiled to WebAssembly
- **📝 Live Preview** - Real-time Typst compilation as you type
- **🎨 Syntax Highlighting** - VSCode Dark+ theme with comprehensive Typst syntax support
- **📄 Multi-page Documents** - Proper rendering of documents with multiple pages
- **📚 Bibliography Support** - Dynamic Hayagriva YAML bibliography with file resolver integration
- **🖼️ Image Gallery** - Upload, manage, and embed images with IndexedDB storage and sequential IDs (001-999)
- **📥 PDF Export** - Download your documents as PDF files
- **💾 Auto-save** - Automatic localStorage persistence of your work
- **🌓 Dark Theme** - Eye-friendly dark editor interface
- **📱 Responsive** - Works on desktop and mobile devices
- **📐 IEEE Templates** - Built-in support for IEEE-style academic papers
- **🔄 Dual Mode Editor** - Switch between source code and visual editing modes

### Deployment Options
- **🌐 Web App** - Run in any modern browser, deploy to GitHub Pages/Netlify
- **🖥️ Desktop App** - Native application for Windows, Linux, and macOS via Tauri
- **📦 Same Codebase** - Single codebase for both web and desktop deployments

## 🛠️ Tech Stack

### Frontend (Shared)
- **[Rust](https://www.rust-lang.org/)** - Systems programming language
- **[Leptos 0.8](https://leptos.dev/)** - Reactive web framework for Rust
- **[typst-as-lib](https://github.com/Myriad-Dreamin/typst.ts)** - Typst compilation engine
- **[Tailwind CSS](https://tailwindcss.com/)** + **[DaisyUI](https://daisyui.com/)** - Styling framework
- **[Iconify](https://iconify.design/)** - Icon library

### Web Deployment
- **[Trunk](https://trunkrs.dev/)** - WASM web application bundler

### Desktop Deployment
- **[Tauri 2.8](https://tauri.app/)** - Native desktop application framework

## 📋 Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (nightly toolchain)
- **wasm32-unknown-unknown** target
- **Trunk** - WASM bundler

### Installation

```bash
# Install Rust nightly
rustup toolchain install nightly --allow-downgrade

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Trunk
cargo install trunk
```

## 🚀 Quick Start

> **📖 TL;DR?** See [QUICK_START.md](./QUICK_START.md) for a concise command reference.

### Web Development

Clone the repository and start the web development server:

```bash
# Install Node dependencies (for Tailwind CSS)
npm install

# Build Tailwind CSS
npm run build-css

# Start web development server
trunk serve --open
```

This will:
- Compile the Rust code to WASM
- Start a local server at `http://localhost:1420`
- Open the app in your default browser
- Watch for file changes and auto-reload

### Desktop Development (Tauri)

Run as a native desktop application:

```bash
# Install dependencies (first time only)
npm install

# Start desktop app with hot-reload
npm run tauri:dev
# or
cargo tauri dev
```

This will:
- Start trunk serve automatically
- Compile the Tauri backend
- Open a native desktop window
- Enable hot-reload for both frontend and backend

### Production Build

#### Web Build

Build an optimized web version:

```bash
# Build for production (GitHub Pages)
trunk build --config Trunk-release.toml

# Or for standard release
trunk build --release
```

The compiled files will be available in the `docs/` directory (GitHub Pages) or `dist/` directory (standard), ready to be deployed to any static hosting service.

**GitHub Pages Deployment**: The project includes a GitHub Actions workflow (`.github/workflows/deploy.yml`) that automatically builds and deploys to GitHub Pages on push to main/master branch.

#### Desktop Build

Build native desktop installers:

```bash
# Build desktop app for your platform
npm run tauri:build
# or
cargo tauri build
```

The installers will be generated in `src-tauri/target/release/bundle/`:
- **Linux**: `.deb`, `.AppImage`, `.rpm`
- **Windows**: `.msi`, `.exe`
- **macOS**: `.dmg`, `.app`

**Installer Size**: ~85-100 MB (includes Tauri runtime + WASM bundle)

## 📁 Project Structure

```
wasm-typst-studio-rs/
├── src/                        # Frontend Leptos app (shared by web & desktop)
│   ├── lib.rs                  # Main application entry point
│   ├── main.rs                 # WASM entry point
│   ├── compiler/
│   │   ├── typst.rs            # Typst compilation wrapper with static file resolver
│   │   └── mod.rs
│   ├── components/             # UI components
│   │   ├── editor.rs
│   │   ├── preview.rs
│   │   ├── image_gallery.rs
│   │   └── mod.rs
│   └── utils/
│       ├── highlight.rs        # Syntax highlighting implementation
│       ├── image_manager.rs    # Image management with sequential IDs
│       ├── image_storage.rs    # IndexedDB for image storage
│       └── mod.rs
├── src-tauri/                  # Tauri backend (desktop only)
│   ├── src/
│   │   ├── main.rs             # Desktop app entry point
│   │   └── lib.rs              # Tauri setup and configuration
│   ├── Cargo.toml              # Backend dependencies
│   ├── tauri.conf.json         # Tauri configuration
│   ├── build.rs                # Build script
│   ├── capabilities/           # Permission system
│   └── icons/                  # App icons for different platforms
├── public/
│   └── favicon.ico
├── examples/
│   ├── example.typ             # Default example document
│   ├── example2.typ            # IEEE template example
│   └── refs.yml                # Bibliography in Hayagriva YAML format
├── .github/
│   └── workflows/
│       └── deploy.yml          # GitHub Pages deployment workflow
├── index.html                  # HTML template (web)
├── tailwind.css                # Tailwind CSS with custom styles
├── tailwind.config.js          # Tailwind configuration
├── Trunk.toml                  # Trunk configuration (development)
├── Trunk-release.toml          # Trunk configuration (production/GitHub Pages)
├── Cargo.toml                  # Frontend Rust dependencies
├── package.json                # Node dependencies (Tailwind + scripts)
├── rust-toolchain.toml         # Rust toolchain version
├── README.md                   # This file
└── TAURI_INTEGRATION.md        # Detailed Tauri integration guide
```

## 🎯 Usage

### Basic Editing

1. Type your Typst markup in the left panel (Source)
2. See the live preview in the right panel (Preview)
3. Your work is automatically saved to localStorage

### Adding Images

1. Click the **"Images"** button in the toolbar to open the Image Gallery
2. Upload an image file (PNG, JPG, JPEG, GIF, WebP, SVG)
3. Images are automatically assigned sequential IDs (001, 002, ..., 999)
4. Click "Copy ID" to copy the image ID
5. Use it in your Typst code:
   ```typst
   #figure(
     image("001.png"),
     caption: [Your image caption]
   )
   ```

**Image Gallery Features:**
- Sequential 3-digit IDs for easy reference
- Preview thumbnails with filename display
- Copy ID and delete operations
- Persistent storage using IndexedDB
- Drag-and-drop upload support

### Managing Bibliography

1. Click the **"Bibliography"** button in the toolbar to open the Bibliography Manager
2. Edit your references in Hayagriva YAML format:
   ```yaml
   netwok2020:
     type: article
     title: "The Challenges of Scientific Typesetting"
     author: ["Network, A.", "Smith, B."]
     date: 2020
     journal: "Journal of Academic Publishing"
   ```
3. Reference them in your document:
   ```typst
   According to @netwok2020, this is correct.

   = References
   #bibliography("refs.yml")
   ```

**How it works:**
- Bibliography is stored in localStorage and registered as a virtual file (`refs.yml`)
- Uses typst-as-lib's static file resolver to provide the file to the compiler
- Supports all Hayagriva YAML entry types (article, book, web, etc.)

### Exporting PDF

Click the **"PDF"** button in the toolbar to download your document as a PDF file.

## 🎨 Typst Syntax Examples

The editor comes pre-loaded with comprehensive examples including:

- **Text Formatting** - Bold, italic, strikethrough, colors, sizes
- **Lists** - Unordered, ordered, nested, term lists
- **Code Blocks** - Syntax-highlighted code in multiple languages
- **Mathematics** - Inline and display formulas, matrices, equations
- **Tables** - Simple and styled tables
- **Figures** - Images and captions with cross-references
- **Advanced Layout** - Columns, boxes, blocks

## 🔧 Configuration

### Trunk.toml (Development)

Development configuration with local URLs:

```toml
[build]
target = "index.html"
dist = "dist"
public_url = "/"

[serve]
port = 3000
```

### Trunk-release.toml (Production)

Production configuration for GitHub Pages:

```toml
[build]
target = "index.html"
dist = "docs"
public_url = "/wasm-typst-studio-rs/"  # Must match your repository name
release = true
minify = "always"
```

To build for GitHub Pages:
```bash
trunk build --config Trunk-release.toml
```

### rust-toolchain.toml

Specify the Rust toolchain version:

```toml
[toolchain]
channel = "nightly-2025-01-01"
targets = ["wasm32-unknown-unknown"]
```

## 🐛 Known Limitations

### Web Version
- **System Fonts** - Not available in WASM; uses embedded fonts only
- **File System** - No direct file system access; uses IndexedDB for images and virtual file resolver for bibliography
- **Large Documents** - Very large documents may experience performance degradation
- **Image Limit** - Maximum 999 images per session (sequential ID constraint)
- **External Resources** - Cannot load external files or packages (all resources must be embedded)

### Desktop Version
- Most web limitations are resolved or can be addressed with Tauri APIs
- **File System Access** - Can be added via Tauri commands (see `TAURI_INTEGRATION.md`)
- **Native Dialogs** - Open/Save dialogs available
- **System Fonts** - Still limited by WASM constraints in current implementation

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Test thoroughly: `trunk serve`
5. Commit your changes: `git commit -m 'Add amazing feature'`
6. Push to the branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

### Development Guidelines

- Follow Rust conventions and use `cargo fmt`
- Ensure all code compiles without warnings: `cargo check`
- Test WASM compilation: `trunk build`
- Keep dependencies minimal and well-justified
- Document public APIs and complex logic

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- **[Typst](https://typst.app)** - The amazing typesetting system
- **[Leptos](https://leptos.dev/)** - Reactive Rust web framework
- **[typst.ts](https://github.com/Myriad-Dreamin/typst.ts)** - Typst WASM integration
- **[Trunk](https://trunkrs.dev/)** - WASM build tooling

## 📚 Resources

### Typst
- [Typst Documentation](https://typst.app/docs)
- [Typst Tutorial](https://typst.app/docs/tutorial/)
- [Hayagriva YAML Format](https://github.com/typst/hayagriva/blob/main/docs/file-format.md)
- [typst-as-lib](https://github.com/Myriad-Dreamin/typst.ts)

### Rust & WASM
- [Leptos Book](https://book.leptos.dev/)
- [Trunk Documentation](https://trunkrs.dev/)
- [WebAssembly Concepts](https://developer.mozilla.org/en-US/docs/WebAssembly)

### Tauri (Desktop)
- [Tauri Documentation](https://v2.tauri.app/)
- [Tauri + Leptos Guide](https://v2.tauri.app/start/frontend/leptos/)
- [TAURI_INTEGRATION.md](./TAURI_INTEGRATION.md) - Detailed integration guide for this project
- [Tauri API Reference](https://v2.tauri.app/reference/javascript/api/)

## 🖥️ Desktop App Details

This project supports both web and desktop deployments from the same codebase. For detailed information about:
- Desktop-specific features
- Tauri configuration
- Native API integration
- File system access
- Building and distributing desktop apps

See **[TAURI_INTEGRATION.md](./TAURI_INTEGRATION.md)** for the complete guide.

## 📖 Documentation Index

### Main Documentation
- **[README.md](./README.md)** - Main documentation (this file)
- **[QUICK_START.md](./QUICK_START.md)** - Quick command reference
- **[TAURI_INTEGRATION.md](./TAURI_INTEGRATION.md)** - Desktop app integration guide

### Build & Optimization Guides
- **[info/BUILD_REQUIREMENTS.md](./info/BUILD_REQUIREMENTS.md)** - System requirements and packages for each OS
- **[info/OPTIMIZATION_REPORT.md](./info/OPTIMIZATION_REPORT.md)** - Complete optimization analysis and recommendations
- **[info/OPTIMIZATIONS_APPLIED.md](./info/OPTIMIZATIONS_APPLIED.md)** - Implemented optimizations details
- **[info/OPTIMIZATIONS_SUMMARY.md](./info/OPTIMIZATIONS_SUMMARY.md)** - Quick optimization results overview

---

**Built with ❤️ using Rust, WebAssembly, and Tauri**
