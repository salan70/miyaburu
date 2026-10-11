# miyaburu / みやぶる

A **Rust-first**, local Markdown preview CLI for documents produced by coding agents.
The goal is to open a Markdown file in a thoughtfully designed browser view
using one command:

```sh
miyaburu docs/plan.md
```

> **Status: early scaffold.** The browser preview command above is the intended
> interface, **not an implemented command**. The repository currently contains
> an initial Rust source-loading function and development documentation.

## Intended experience

- White, digital **Editorial × Soft Pop** presentation without paper texture.
- LINE Seed JP for UI and prose; HackGen for code.
- Visible Markdown heading markers (`##`) and list markers (`-`), but no
  formatting-only delimiters like `**` or inline-code backticks.
- Visual line breaks after Japanese `。` in normal prose and list items.
- Short labels for long file paths and URLs, without losing the original
  destination or copyable value.
- GFM, code highlighting and Mermaid, ultimately offline-first.
- Customizable from a local config file without writing CSS.
- Simple integration with Codex, Claude Code and other agent workflows.

The initial work is scoped to desktop local preview. Phone access (potentially
through Cloudflare Tunnel with authentication) is an idea for a later version.

## For contributors

Rust is the product language. Node.js >=22 is needed **only** to run
[DocBridge](https://github.com/salan70/docbridge) while developing; it is not
intended to be a runtime requirement for users.

```sh
cargo test

npm install
npm run docs:check
npm run docs:audit
```

This development scaffold has **not yet been executed or verified** in the
authoring environment. Commit an npm lockfile after resolving dependencies
locally so CI can use `npm ci`.

The configuration is in [docbridge.config.json](docbridge.config.json).
A real pair of reciprocal annotations connects
[`src/lib.rs`](src/lib.rs) with
[the input contract](docs/specs/input.md).

Read [the rendering contract](docs/specs/rendering.md) and
[AGENTS.md](AGENTS.md) before implementing browser preview. As features are
implemented, link concrete Rust declarations to the appropriate specification
sections with DocBridge. Avoid creating links to non-existent code.

## Design references

- [UI/UX NUMA](https://github.com/salan70/uiux-numa) — typography,
  spacing, color roles and design evaluations.
- [DocBridge](https://github.com/salan70/docbridge) — development-time
  documentation/code consistency.
