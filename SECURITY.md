# Security Policy

## Scope

This repository contains a single-player Rust/Bevy life-sim game that also
ships a WebAssembly build playable in the browser at
<https://rodmen07.github.io/new_game/>.

The game has no accounts, no payments, and stores no personal data. The
realistic security surface is:

- The browser front-end (`index.html`) and the deployed WASM bundle.
- Save data persisted to the local filesystem (native) or `localStorage`
  (browser).
- The optional WebSocket position-sync to the multiplayer relay defined in
  `src/network.rs`.
- The repository's CI / GitHub Actions configuration.

The relay server itself (Fly.io deployment) is **out of scope** for this
repository — only the in-repo client code is in scope.

## Supported Versions

Only the latest commit on the `main` branch and the most recent published
GitHub Pages deployment receive security fixes. There are no long-lived
release branches.

| Version           | Supported          |
| ----------------- | ------------------ |
| `main` (latest)   | :white_check_mark: |
| older commits     | :x:                |

## Reporting a Vulnerability

Please report vulnerabilities **privately** rather than opening a public
issue. The preferred channel is GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository:

1. Go to the repository's **Security** tab.
2. Click **Report a vulnerability**.
3. Fill out the advisory form with reproduction steps, affected files /
   commits, and the impact you observed.

Please include:

- A description of the vulnerability and a minimal reproduction.
- The commit SHA or deployment URL where the issue was observed.
- Any logs, save files, or network captures relevant to the report
  (redacted of personal data).

### What to expect

- Acknowledgement of the report within **7 days**.
- A status update or triage decision within **30 days**.
- For accepted reports, a fix and (where applicable) a published GitHub
  Security Advisory crediting the reporter unless they prefer to remain
  anonymous.

Reports that turn out not to be in scope (for example, vulnerabilities in
upstream `bevy`, `serde_json`, or other dependencies) will be redirected to
the appropriate upstream project.
