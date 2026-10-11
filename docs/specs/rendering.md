# Rendering contract (planned)

This is an implementation target, not a description of a working renderer.

## Presentation

- Default appearance: white, digital, Editorial × Soft Pop; restrained color.
- Body, headings and UI: LINE Seed JP. Code and inline code: HackGen.
- Preserve heading markers such as `##` and list markers such as `-` visually.
- Do not display syntax-only formatting delimiters such as `**` and backticks.
  Literal punctuation in the source must remain visible.
- Insert visual line breaks after Japanese `。` in prose, including list items.
  Do not modify code blocks, inline code, URLs, or the original file.
  Do not generate duplicate breaks at the end of a paragraph.
- For ordinary prose, abbreviate long file paths to a filename and long bare
  URLs to an identifiable short label. Preserve the full target for navigation
  and an explicit copy action; never truncate code or command examples.
- Support common Markdown/GFM constructs and Mermaid. Mermaid assets should
  work offline and use safe defaults.

## User experience

The intended command is `miyaburu path/to/document.md`, opening the document
in the default browser without blocking the calling agent unnecessarily.

Defaults should work without configuration. Customization should be possible
with a local TOML configuration file and without requiring hand-written CSS.

## Security and scope

- Never treat arbitrary Markdown as trusted executable HTML.
- Start with loopback-only access and no implicit external exposure.
- The Cloudflare Tunnel / phone-access idea is outside the initial MVP.
- A working preview CLI, asset bundling, and precise configuration schema
  have **not** been implemented or finalized.

## References

- [UI/UX NUMA](https://github.com/salan70/uiux-numa): typography, spacing,
  role-based colors and accessibility principles.
- [DocBridge](https://github.com/salan70/docbridge): link this contract to
  implementations as they are added. Only add reciprocal links when code exists.
