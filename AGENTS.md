# Agent guidance

This repository will implement **miyaburu**, a Rust Markdown preview CLI.
Its public README and `docs/specs/` describe the current contract.
Do not treat planned features as implemented.

## Before changing behavior

- Read the relevant `docs/specs/` section first.
- Consult [uiux-numa](https://github.com/salan70/uiux-numa) for design
  principles and reusable tokens, without importing its entire catalog.
- Keep the user-selected Rust architecture and local/offline-first model.
- Follow the existing visual direction: white digital Editorial × Soft Pop,
  LINE Seed JP for prose and HackGen for code.

## DocBridge (dogfooding)

DocBridge is a **development-time** tool; it must not become a runtime
dependency of the Rust executable.

- Add `/// @doc docs/specs/<file>.md#<section>` on appropriate public Rust
  declarations and reciprocal `<!-- @code src/<file>.rs#<Symbol> -->`
  annotations immediately above matching Markdown headings.
- Prefer linking real user-visible contracts, not every internal helper.
- After changing linked code or docs, run `npm run docs:check`.
- Run `npm run docs:audit` periodically to find candidates, not to force
  meaningless one-to-one links.
- When updating an annotated declaration, review its linked specification.
- DocBridge requires Node.js >=22 **only for contributors and CI**.
  End users should need just the compiled Rust executable.

## Validation

After making Rust changes, run `cargo fmt --check` and `cargo test`.
For local DocBridge checks, run `npm install` once, then `npm run docs:check`.

No working browser preview is claimed by this initial scaffold.
