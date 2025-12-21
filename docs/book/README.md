# Mycelium Documentation

This directory contains the complete documentation for Mycelium API Gateway, built with [mdBook](https://rust-lang.github.io/mdBook/).

## 📚 Building the Documentation

### Prerequisites

Install mdBook:

```bash
cargo install mdbook
```

### Build Commands

**Serve locally with live reload:**
```bash
cd docs/book
mdbook serve --open
```

**Build for production:**
```bash
cd docs/book
mdbook build
```

**Clean build artifacts:**
```bash
cd docs/book
mdbook clean
```

## 🎨 Customization

The documentation includes:

- **Custom CSS** (`theme/custom.css`) - Brand colors, enhanced navigation, responsive design
- **Favicon** (`theme/favicon.svg`) - Mycelium logo
- **Configuration** (`book.toml`) - mdBook settings

## 📖 Structure

```
docs/book/
├── book.toml              # mdBook configuration
├── src/
│   ├── SUMMARY.md         # Table of contents
│   ├── 00-introduction.md
│   ├── 01-authorization.md
│   ├── 02-installation.md
│   ├── 03-quick-start.md
│   ├── 04-configuration.md
│   ├── 05-deploy-locally.md
│   ├── 06-downstream-apis.md
│   └── 07-running-tests.md
├── theme/
│   ├── custom.css         # Custom styles
│   ├── favicon.svg        # Icon
│   └── favicon.png        # PNG fallback
└── book/                  # Generated HTML (gitignored)
```

## 🚀 Deployment

### GitHub Pages

The documentation can be automatically deployed to GitHub Pages using the included workflow (`.github/workflows/deploy-docs.yml`).

**To enable:**

1. Go to repository Settings → Pages
2. Select "GitHub Actions" as the source
3. Push changes to `main` branch

The documentation will be available at: `https://lepistabioinformatics.github.io/mycelium/`

### Manual Deployment

Build and copy the `book` directory to your web server:

```bash
cd docs/book
mdbook build
# Copy ./book/* to your server
```

## 🎯 Features

- ✅ Dark theme by default
- ✅ Integrated search
- ✅ Responsive design
- ✅ Custom navigation with Previous/Next buttons
- ✅ Syntax highlighting for code blocks
- ✅ Print-friendly
- ✅ Edit on GitHub links
- ✅ Custom brand styling

## 📝 Contributing

To add or modify documentation:

1. Edit files in `src/`
2. Update `SUMMARY.md` if adding new pages
3. Test locally with `mdbook serve`
4. Commit and push changes

## 🔗 Links

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [Mycelium Repository](https://github.com/LepistaBioinformatics/mycelium)
- [Report Documentation Issues](https://github.com/LepistaBioinformatics/mycelium/issues)
