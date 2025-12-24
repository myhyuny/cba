# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust CLI tool for creating CBZ (Comic Book Archive) files from image directories with intelligent Zopfli compression.

**Key features:**
- Natural sorting by numeric values in filenames (regex-based)
- Parallel processing with rayon
- Per-image compression decision (compressed vs uncompressed, whichever is smaller)
- Single-file design in [src/main.rs](src/main.rs)

## Commands

```shell
# Build
cargo build --release

# Run
cargo run -- [directory...]

# Example
cargo run -- ./manga/chapter1 ./manga/chapter2
```

## Architecture

### Processing Pipeline

1. **Input validation** ([main.rs:40-56](src/main.rs#L40-L56)): Collect image files (avif, gif, jpg, jpeg, png, webp)
2. **Natural sorting** ([main.rs:61-86](src/main.rs#L61-L86)): Sort by extracted numeric values
3. **Parallel compression** ([main.rs:90-143](src/main.rs#L90-L143)): Each image compressed with Zopfli (level 264, 1MB buffer), size comparison
4. **Final assembly** ([main.rs:145-164](src/main.rs#L145-L164)): Combine into CBZ with optimal storage method

### Important Details

- **Compression settings** ([main.rs:108-111](src/main.rs#L108-L111)): Level 264, 1MB buffer (tuned values)
- **Windows UTF-8** ([main.rs:32-36](src/main.rs#L32-L36)): Console output set to UTF-8 for Korean filenames
- **Lints**: `clippy::needless_return` disabled - explicit returns preferred
