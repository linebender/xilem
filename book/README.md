# The Xilem Book

This directory contains the source files for **The Xilem Book**, a comprehensive guide to learning and using Xilem.

## Building the Book

The book is built using [mdBook](https://rust-lang.github.io/mdBook/), a utility for creating books from Markdown files.

### Requirements

- mdBook version 0.5.1 or higher

### Installation

Install mdBook using Cargo:
```bash
cargo install mdbook
```

Or download pre-built binaries from the [mdBook releases page](https://github.com/rust-lang/mdBook/releases).

### Building Locally

To build the book and generate the HTML output:
```bash
cd book
mdbook build
```

The generated HTML files will be in the `book/` subdirectory.

### Development Mode

For local development with live reload:
```bash
cd book
mdbook serve
```

This will start a local web server (typically at `http://localhost:3000`) and automatically rebuild the book when you make changes to the source files.

### Opening the Built Book

After running `mdbook serve`, open your browser to the URL shown in the terminal output (usually `http://localhost:3000`).

## Structure

The book's content is organized in the `src/` directory:

- `SUMMARY.md` - Defines the book's table of contents and structure
- Numbered directories (e.g., `01-getting-started/`) - Contain chapter content organized by topic
- `assets/` - Images and other static resources

## Contributing

Contributions to The Xilem Book are welcome! Please see the main repository's contributing guidelines for more information.

Most chapters are currently placeholders waiting for content. Feel free to pick a chapter and start writing, or open an issue to discuss your contribution plans.