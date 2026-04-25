"""
Tool implementations exposed to the gamedev agent.

Four tools: read_file, list_dir, write_file, run_shell.
All paths are relative to the new_game repo root (the workspace root in CI).
"""

from __future__ import annotations

import pathlib
import subprocess
from typing import Any

# ---------------------------------------------------------------------------
# Workspace root
# ---------------------------------------------------------------------------

# agents/gamedev/tools.py -> agents/gamedev -> agents -> repo root
REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent.parent

# ---------------------------------------------------------------------------
# Guard lists
# ---------------------------------------------------------------------------

_FORBIDDEN_WRITE_SUBSTRINGS = [
    "Cargo.toml",
    "Cargo.lock",
    ".github/",
    "scripts/",
    "audits/",
    "docs/",
    "save.json",
    "README.md",
    "SECURITY.md",
    "config.toml",
    "index.html",
    "target/",
    "agents/",
]

# Allow read-only inspection commands and the standard Rust toolchain
# verification commands. Anything else is rejected.
_ALLOWED_SHELL_PREFIXES = (
    "cargo check",
    "cargo fmt --all -- --check",
    "cargo fmt --all",
    "cargo clippy --all-targets -- -D warnings",
    "cargo clippy",
    "cargo test --all-targets",
    "cargo test",
    "cargo build",
    "grep ",
    "rg ",
    "find ",
    "ls",
    "cat ",
    "head ",
    "tail ",
    "wc ",
)

_BLOCKED_SHELL_SUBSTRINGS = (
    "git ",
    "rm ",
    "mv ",
    "cp ",
    "curl ",
    "wget ",
    "sudo ",
    "ssh ",
    "scp ",
    "pip install",
    "cargo install",
    "rustup ",
    ">",  # no redirection
    "|",  # no piping (avoids shell escapes)
    "`",
    "$(",
    "&&",
    ";",
)


# ---------------------------------------------------------------------------
# Tool implementations
# ---------------------------------------------------------------------------

def _within_repo(p: pathlib.Path) -> bool:
    try:
        p.resolve().relative_to(REPO_ROOT)
        return True
    except ValueError:
        return False


def read_file(args: dict[str, Any]) -> str:
    path = str(args.get("path", "")).strip()
    if not path:
        return "ERROR: missing 'path' argument"
    target = (REPO_ROOT / path).resolve()
    if not _within_repo(target):
        return f"ERROR: path escapes the repo root: {path}"
    try:
        text = target.read_text(encoding="utf-8")
    except FileNotFoundError:
        return f"ERROR: file not found: {path}"
    except UnicodeDecodeError:
        return f"ERROR: file is not UTF-8 text: {path}"
    except OSError as exc:
        return f"ERROR reading {path}: {exc}"
    # Cap response size to keep the model context bounded.
    if len(text) > 200_000:
        return text[:200_000] + f"\n\n[TRUNCATED: file is {len(text)} bytes]"
    return text


def list_dir(args: dict[str, Any]) -> str:
    path = str(args.get("path", ".")).strip() or "."
    target = (REPO_ROOT / path).resolve()
    if not _within_repo(target):
        return f"ERROR: path escapes the repo root: {path}"
    if not target.is_dir():
        return f"ERROR: not a directory: {path}"
    entries = []
    for child in sorted(target.iterdir()):
        suffix = "/" if child.is_dir() else ""
        entries.append(f"{child.name}{suffix}")
    return "\n".join(entries) if entries else "(empty)"


def write_file(args: dict[str, Any], allowed_paths: list[str]) -> str:
    path = str(args.get("path", "")).strip()
    content = args.get("content", "")
    if not path:
        return "ERROR: missing 'path' argument"
    if not isinstance(content, str):
        return "ERROR: 'content' must be a string"

    if path not in allowed_paths:
        return (
            f"ERROR: path '{path}' is not in the focus area's allowed write list. "
            f"Allowed: {allowed_paths}"
        )
    for forbidden in _FORBIDDEN_WRITE_SUBSTRINGS:
        if forbidden in path:
            return f"ERROR: writes to paths containing '{forbidden}' are forbidden by policy."

    target = (REPO_ROOT / path).resolve()
    if not _within_repo(target):
        return f"ERROR: path escapes the repo root: {path}"

    # Sanitize Unicode smart quotes and em dashes that LLMs sometimes emit.
    content = (
        content
        .replace("\u201c", '"')
        .replace("\u201d", '"')
        .replace("\u2018", "'")
        .replace("\u2019", "'")
        .replace("\u2014", "-")
        .replace("\u2013", "-")
    )

    try:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    except OSError as exc:
        return f"ERROR writing {path}: {exc}"
    return f"OK: wrote {len(content)} bytes to {path}"


def run_shell(args: dict[str, Any]) -> str:
    command = str(args.get("command", "")).strip()
    if not command:
        return "ERROR: missing 'command' argument"

    for blocked in _BLOCKED_SHELL_SUBSTRINGS:
        if blocked in command:
            return (
                f"ERROR: command contains blocked token '{blocked.strip()}'. "
                "Use read_file / write_file for file ops; only `cargo` and "
                "read-only inspection commands are permitted."
            )
    if not any(command.startswith(p) for p in _ALLOWED_SHELL_PREFIXES):
        return (
            "ERROR: only `cargo` toolchain commands and read-only inspection "
            "commands (grep, rg, find, ls, cat, head, tail, wc) are allowed."
        )

    try:
        result = subprocess.run(
            command,
            shell=True,
            capture_output=True,
            text=True,
            cwd=str(REPO_ROOT),
            timeout=600,
        )
    except subprocess.TimeoutExpired:
        return "ERROR: command timed out after 10 minutes"
    except OSError as exc:
        return f"ERROR running shell command: {exc}"

    output = (result.stdout + result.stderr)[:20_000]
    return f"exit={result.returncode}\n{output or '(no output)'}"


# ---------------------------------------------------------------------------
# Tool schema (OpenAI / GitHub Models tool-calling format)
# ---------------------------------------------------------------------------

def tool_schemas() -> list[dict]:
    """Return the OpenAI/GitHub Models tool schema list."""
    return [
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": (
                    "Read the complete contents of a UTF-8 text file in the new_game repo. "
                    "Paths are relative to the repo root."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Repo-relative path, e.g. 'src/systems/visual.rs'.",
                        }
                    },
                    "required": ["path"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List the contents of a directory in the repo.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Repo-relative directory path.",
                        }
                    },
                    "required": ["path"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": (
                    "Overwrite a file with the COMPLETE new contents. The path MUST be one of "
                    "the focus area's allowed_paths from the user prompt; any other path is "
                    "rejected. Provide the entire file, not a diff."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                    },
                    "required": ["path", "content"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "run_shell",
                "description": (
                    "Run one shell command from the repo root. Allowed: cargo "
                    "(check/fmt/clippy/test/build) and read-only inspection (grep, rg, find, "
                    "ls, cat, head, tail, wc). All other commands are rejected."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"},
                    },
                    "required": ["command"],
                },
            },
        },
    ]


def make_dispatch(allowed_paths: list[str]):
    """Return a name -> callable map suitable for the agent loop."""
    return {
        "read_file": read_file,
        "list_dir": list_dir,
        "write_file": lambda a: write_file(a, allowed_paths),
        "run_shell": run_shell,
    }
