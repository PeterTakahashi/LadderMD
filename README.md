# LadderMD

**PLCopen XML to Markdown/DSL converter for ladder diagrams**

[Japanese / 日本語](README_JA.md)

<p align="center">
  <img src="docs/images/ladder-concept.svg" alt="LadderMD concept: PLCopen XML to LadderMD to Roundtrip validation" width="780"/>
</p>

## Why LadderMD

### PLC ladder diagrams are locked in proprietary silos

PLC ladder diagrams are the backbone of factory automation worldwide. Yet they remain trapped inside vendor-specific binary formats -- Mitsubishi GX Works, Omron Sysmac, Siemens TIA Portal, and others each use their own closed file format. You cannot `git diff` a ladder diagram. You cannot `grep` for a variable name across a project. You cannot review control logic changes in a pull request.

There is a cultural divide at play. The people who write ladder diagrams are electrical/controls engineers ("electricians" in the Japanese industry vernacular), not software engineers. The tools they use reflect this: graphical, proprietary, and disconnected from modern development workflows.

### LLMs and AI agents cannot read control logic

Modern AI agents process text -- Markdown, code, structured data. Cloudflare's [Markdown for Agents](https://blog.cloudflare.com/markdown-for-agents/) initiative demonstrates the direction: all content should be representable as text for machine consumption.

PLC ladder diagrams have no such text representation. PLCopen XML (IEC 61131-10) exists as an international interchange format, but raw XML is verbose, burning tokens on angle brackets and namespace declarations rather than conveying logic. There is currently no compact, readable text format that an LLM can consume to understand what a ladder circuit does.

### Knowledge transfer is failing

In manufacturing, it is common to find ladder programs written 20-30 years ago, still running in production, with the original author long retired. When these programs need modification, the institutional knowledge of _why_ the circuit was designed a certain way is often lost.

If ladder diagrams existed as text, an LLM could explain them: "This is a self-hold circuit with an emergency stop interlock. The NC contact on X010 ensures fail-safe behavior." But today, these diagrams can only be viewed in each vendor's proprietary software, making automated analysis impossible.

## What LadderMD Does

LadderMD converts PLC ladder diagrams from PLCopen XML (the international standard interchange format) into human- and AI-readable Markdown. It verifies logical equivalence through roundtrip conversion (XML -> Model -> XML). The core library is CLI-independent, designed for extension into Web APIs, desktop apps, and language bindings. Written in Rust, it parses basic circuits in ~20 microseconds.

## Features

- **Parse** PLCopen XML (TC6 v2.01) ladder diagrams into a typed internal model
- **Render** to Markdown with logic expressions, device tables, and ASCII art diagrams
- **Write** back to PLCopen XML from the internal model
- **Validate** roundtrip equivalence (XML -> Model -> XML -> Model)
- **Fast** -- parses basic circuits in ~20 microseconds (zero-copy with `quick-xml`)
- **Library-first** -- core logic is a standalone crate, ready for Web API / desktop / bindings

## Demo

### `laddermd convert`

<p align="center">
  <img src="docs/images/demo-convert.svg" alt="laddermd convert demo" width="820"/>
</p>

### `laddermd validate`

<p align="center">
  <img src="docs/images/demo-validate.svg" alt="laddermd validate demo" width="720"/>
</p>

## Installation

```bash
# Clone and build
git clone https://github.com/yourname/laddermd.git
cd laddermd
cargo build --release

# The binary is at target/release/laddermd-cli
```

Requires Rust 1.75+ (stable).

## Usage

### Convert PLCopen XML to Markdown

```bash
# Output to stdout
laddermd convert input.xml

# Output to file
laddermd convert input.xml -o output.md

# Customize output (disable sections)
laddermd convert input.xml --no-diagram
laddermd convert input.xml --no-table
laddermd convert input.xml --no-logic
```

### Show project info

```bash
$ laddermd info input.xml
Project: SelfHoldTest
  Program: Main
    Rungs: 2
    Contacts: 4, Coils: 2, Blocks: 0
```

### Validate roundtrip conversion

```bash
$ laddermd validate input.xml
Parse OK: 2 rungs found
Devices: 4 contacts, 2 coils, 0 blocks
Roundtrip OK: all rungs logically equivalent
```

## Output Format (LadderMD)

Each rung is rendered with three sections:

**1. Logic expression** -- Boolean formula describing the rung

```
LOGIC: Y001 = (X001 AND X002 OR Y001)
```

**2. Device table** -- All contacts, coils, and blocks in the rung

```
| Device | Type        | LocalId |
|--------|-------------|---------|
| X001   | Contact(NO) | 2       |
| X002   | Contact(NO) | 3       |
| Y001   | Coil        | 5       |
```

**3. ASCII ladder diagram** -- Visual representation

```
|--[X001]--[X002]--+--(Y001)|
|--[Y001]--+        |
```

### Symbol Reference

| Symbol | Meaning |
|--------|---------|
| `[X001]` | Normally Open contact (NO / a-contact) |
| `[/X001]` | Normally Closed contact (NC / b-contact) |
| `(Y001)` | Output coil |
| `(S Y001)` | Set (latch) coil |
| `(R Y001)` | Reset (unlatch) coil |
| `[TON T1]` | Function block (e.g., timer) |
| `--+--` | Parallel branch junction (OR) |

## Supported Circuits

| Circuit | Description | Test Fixture |
|---------|-------------|--------------|
| Self-hold | Latch with seal-in contact + reset | `self_hold.xml` |
| Interlock | Mutual exclusion via NC contacts | `interlock.xml` |
| Timer | TON on-delay timer block | `timer.xml` |
| Emergency stop | NC emergency stop + self-hold | `emergency_stop.xml` |
| Counter | CTU (count up) and CTD (count down) blocks | `counter.xml` |
| Comparison / Arithmetic | GT, EQ, ADD and other function blocks | `comparison.xml` |

## Architecture

```
laddermd/
├── crates/
│   ├── laddermd-core/       # Library crate (CLI-independent)
│   │   └── src/
│   │       ├── model.rs     # Internal data model
│   │       ├── parser/      # PLCopen XML -> Model
│   │       ├── renderer/    # Model -> Markdown
│   │       ├── writer/      # Model -> PLCopen XML
│   │       └── validator/   # Roundtrip equivalence check
│   └── laddermd-cli/        # CLI binary
└── tests/fixtures/          # PLCopen XML test files
```

The core library (`laddermd-core`) has no CLI dependencies and exposes a public API:

```rust
use laddermd_core::{parser, renderer::MarkdownRenderer, writer, validator};

// Parse
let project = parser::parse(&xml_string)?;

// Render to Markdown
let renderer = MarkdownRenderer::default();
let markdown = renderer.render(&project);

// Write back to XML
let xml_output = writer::write(&project)?;

// Validate roundtrip
let result = validator::validate(&xml_string)?;
assert!(result.roundtrip_ok);
```

## Benchmarks

Run with `cargo bench`:

| Fixture | Parse Time |
|---------|-----------|
| self_hold.xml | ~24 us |
| interlock.xml | ~22 us |
| timer.xml | ~21 us |
| emergency_stop.xml | ~16 us |

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy

# Run benchmarks
cargo bench
```

## Vision / Roadmap

### Near-term (v0.x)

- [x] Basic circuits: self-hold, interlock, timer, emergency stop
- [x] Counter blocks (CTU, CTD)
- [x] Comparison and arithmetic blocks (GT, GE, EQ, LE, LT, NE, ADD, SUB, MUL, DIV, MOD)

### Mid-term (v1.x)

- [x] Mitsubishi GX Works mnemonic format input -- covering the dominant PLC platform in Japanese manufacturing
- [ ] MCP (Model Context Protocol) server -- enabling AI agents to read and write ladder diagrams directly
- [ ] Web API (axum) -- conversion as a service

### Long-term

- [ ] Desktop viewer (Tauri)
- [ ] Python bindings (PyO3) / Node.js bindings (napi-rs)
- [ ] LLM-powered circuit analysis and safety verification -- e.g., "Does this emergency stop circuit have any gaps?" or "Is this interlock correctly implemented?"

## License

MIT
