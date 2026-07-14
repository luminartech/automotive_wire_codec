# Security Policy

## Supported Versions

Security fixes are applied to the latest release on [crates.io](https://crates.io/crates/automotive-wire-codec).
Older releases do not receive backported fixes.

## Reporting a Vulnerability

Please **do not** open a public issue for suspected vulnerabilities.

Report privately via GitHub's private vulnerability reporting:
[Report a vulnerability](https://github.com/luminartech/automotive_wire_codec/security/advisories/new)
(Security tab → *Report a vulnerability*).

You should receive an acknowledgement within 5 business days. Once a fix is
available we will publish a patched release and a GitHub security advisory,
crediting the reporter unless you prefer otherwise.

## Scope

This crate is a `no_std`, no-alloc, `#![forbid(unsafe_code)]` codec-trait
foundation. Reports of particular interest:

- Panics reachable from untrusted input through the `read_*`/`decode` paths
  (the API contract is that malformed input yields `Err`, never a panic).
- `Encode::encoded_size` / `Encode::encode` disagreements that could cause
  buffer-sizing bugs in downstream protocol crates.
- Supply-chain issues in the dependency tree.
