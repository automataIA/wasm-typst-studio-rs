# Typst Studio WASM

A modern, web-based [Typst](https://typst.app) editor built entirely in Rust using WebAssembly. Write and preview Typst documents directly in your browser with real-time compilation, syntax highlighting, and multi-page support.

## ✨ Features

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

## 🛠️ Tech Stack

- **[Rust](https://www.rust-lang.org/)** - Systems programming language
- **[Leptos 0.8](https://leptos.dev/)** - Reactive web framework for Rust
- **[Trunk](https://trunkrs.dev/)** - WASM web application bundler
- **[typst-as-lib](https://github.com/Myriad-Dreamin/typst.ts)** - Typst compilation engine
- **[Tailwind CSS](https://tailwindcss.com/)** + **[DaisyUI](https://daisyui.com/)** - Styling framework
- **[Iconify](https://iconify.design/)** - Icon library

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

### Development

Clone the repository and start the development server:

```bash
# Install Node dependencies (for Tailwind CSS)
npm install

# Build Tailwind CSS
npm run build-css

# Start development server
trunk serve --open
```

This will:
- Compile the Rust code to WASM
- Start a local server at `http://localhost:3000`
- Open the app in your default browser
- Watch for file changes and auto-reload

### Production Build

Build an optimized release version:

```bash
# Build for production (GitHub Pages)
trunk build --config Trunk-release.toml

# Or for standard release
trunk build --release
```

The compiled files will be available in the `docs/` directory (GitHub Pages) or `dist/` directory (standard), ready to be deployed to any static hosting service.

### GitHub Pages Deployment

The project includes a GitHub Actions workflow (`.github/workflows/deploy.yml`) that automatically builds and deploys to GitHub Pages on push to main/master branch.

## 📁 Project Structure

```
wasm-typst-studio-rs/
├── src/
│   ├── lib.rs                  # Main application entry point
│   ├── compiler/
│   │   ├── typst.rs            # Typst compilation wrapper with static file resolver
│   │   └── mod.rs
│   └── utils/
│       ├── highlight.rs        # Syntax highlighting implementation
│       ├── image_manager.rs    # Image management with sequential IDs
│       ├── image_storage.rs    # IndexedDB for image storage
│       └── mod.rs
├── public/
│   └── favicon.ico
├── info/
│   ├── example.typ             # Default example document
│   ├── example2.typ            # IEEE template example
│   └── refs.yml                # Bibliography in Hayagriva YAML format
├── .github/
│   └── workflows/
│       └── deploy.yml          # GitHub Pages deployment workflow
├── index.html                  # HTML template
├── tailwind.css                # Tailwind CSS with custom styles
├── tailwind.config.js          # Tailwind configuration
├── Trunk.toml                  # Trunk configuration (development)
├── Trunk-release.toml          # Trunk configuration (production/GitHub Pages)
├── Cargo.toml                  # Rust dependencies
├── package.json                # Node dependencies (Tailwind)
└── rust-toolchain.toml         # Rust toolchain version
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

- **System Fonts** - Not available in WASM; uses embedded fonts only
- **File System** - No direct file system access; uses IndexedDB for images and virtual file resolver for bibliography
- **Large Documents** - Very large documents may experience performance degradation
- **Image Limit** - Maximum 999 images per session (sequential ID constraint)
- **External Resources** - Cannot load external files or packages (all resources must be embedded)

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

- [Typst Documentation](https://typst.app/docs)
- [Typst Tutorial](https://typst.app/docs/tutorial/)
- [Hayagriva YAML Format](https://github.com/typst/hayagriva/blob/main/docs/file-format.md)
- [Leptos Book](https://book.leptos.dev/)
- [Trunk Documentation](https://trunkrs.dev/)
- [typst-as-lib](https://github.com/Myriad-Dreamin/typst.ts)
- [WebAssembly Concepts](https://developer.mozilla.org/en-US/docs/WebAssembly)

---

**Built with ❤️ using Rust and WebAssembly**
