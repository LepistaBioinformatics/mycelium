# Yew Trunk Template

This is a fairly minimal template for a Yew app that's built with [Trunk].

## Usage

### Installation

If you don't already have it installed, it's time to install Rust: <https://www.rust-lang.org/tools/install>.
The rest of this guide assumes a typical Rust installation which contains both `rustup` and Cargo.

To compile Rust to WASM, we need to have the `wasm32-unknown-unknown` target installed.
If you don't already have it, install it with the following command:

```bash
rustup target add wasm32-unknown-unknown
```

Now that we have our basics covered, it's time to install the star of the show: [Trunk].
Simply run the following command to install it:

```bash
cargo install trunk wasm-bindgen-cli
```

That's it, we're done!

### Running

```bash
trunk serve
```

Rebuilds the app whenever a change is detected and runs a local server to host it.

There's also the `trunk watch` command which does the same thing but without hosting it.

### Development

```bash
trunk build \
    --dist ports/app/dist \
    --all-features ports/app/index.html
```

### Release

```bash
trunk build
    --dist ports/app/dist \
    --all-features ports/app/index.html \
    --release
```
