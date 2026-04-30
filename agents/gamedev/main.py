"""
Gamedev agent - main orchestration loop for rodmen07/new_game.

Each run:
  1. Load state (which dimension/focus pairs are done).
  2. Pick the next task in dimension-first interleaved order.
  3. Create a feature branch off origin/main.
  4. Run the GitHub Models tool-calling loop (default: anthropic/claude-sonnet-4-5).
  5. Verify cargo fmt + clippy + test inside the agent's tool calls.
  6. On success: re-verify from the orchestrator, commit, write PR
     metadata for the workflow step to push + open a PR.
  7. Persist updated state.

Exit codes (read by the GitHub Actions wrapper):
  0 = task committed; output file written; push + PR needed
  1 = unrecoverable error; stop the loop
  2 = task skipped (no changes needed); continue to next task
  3 = all tasks complete; stop

Environment:
  GITHUB_TOKEN          required, used both for git push (via the workflow) and
                        for GitHub Models inference
  GAMEDEV_AGENT_MODEL   optional, defaults to "anthropic/claude-sonnet-4-5"
  GAMEDEV_AGENT_BASE    optional, defaults to "https://models.github.ai/inference"
  FORCE_DIMENSION       optional, force a dimension id (e.g. "graphics")
  FORCE_FOCUS           optional, force a focus id (must be paired with FORCE_DIMENSION)
"""

from __future__ import annotations

import datetime
import json
import logging
import os
import pathlib
import subprocess
import sys

from anthropic import Anthropic

from prompts import SYSTEM_PROMPT, build_task_prompt
from tasks import DIMENSIONS, build_task_queue, pick_next_task
from tools import REPO_ROOT, make_dispatch, tool_schemas

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-8s  %(name)s  %(message)s",
)
log = logging.getLogger("gamedev-agent")

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

AGENT_DIR   = pathlib.Path(__file__).resolve().parent
STATE_FILE  = AGENT_DIR / "state.json"
OUTPUT_FILE = AGENT_DIR / ".gamedev-output.json"
MAX_TOOL_ROUNDS = 25

DEFAULT_MODEL = "anthropic/claude-sonnet-4-5"
DEFAULT_BASE  = "https://models.github.ai/inference"

EXIT_COMMITTED = 0
EXIT_ERROR     = 1
EXIT_SKIP      = 2
EXIT_DONE      = 3


# ---------------------------------------------------------------------------
# State
# ---------------------------------------------------------------------------

def load_state() -> dict:
    if STATE_FILE.exists():
        return json.loads(STATE_FILE.read_text(encoding="utf-8"))
    return {"completed": [], "recent_summaries": [], "last_run": None, "last_pr": None}


def save_state(state: dict) -> None:
    STATE_FILE.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")


def record_completion(state: dict, dim: str, focus: str, summary: str | None = None) -> None:
    state.setdefault("completed", []).append([dim, focus])
    state["last_run"] = datetime.datetime.now(datetime.UTC).isoformat()
    if summary and not summary.upper().startswith("SKIP:"):
        summaries = state.setdefault("recent_summaries", [])
        summaries.append({"dimension": dim, "focus": focus, "summary": summary.strip()})
        state["recent_summaries"] = summaries[-10:]


# ---------------------------------------------------------------------------
# Git helpers (run inside the repo root - the workspace itself in CI)
# ---------------------------------------------------------------------------

def git(*args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        check=check,
    )


def branch_name(dim: str, focus: str) -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    return f"agent/gamedev/{date}/{dim}/{focus}"


def create_branch(branch: str) -> None:
    log.info("Fetching origin/main")
    git("fetch", "origin", "main")
    git("checkout", "-B", branch, "origin/main")
    log.info("Created branch: %s", branch)


def commit_changes(dim: str, focus: str, allowed_paths: list[str]) -> None:
    # Stage only the files the agent was permitted to write. This prevents
    # accidental staging of unrelated files left over in the workspace.
    for p in allowed_paths:
        git("add", p, check=False)
    msg = (
        f"feat({dim}): gamedev agent - {focus}\n\n"
        f"Automated improvement applied by the gamedev agent.\n"
        f"Dimension: {dim}\n"
        f"Focus: {focus}\n"
    )
    git("commit", "-m", msg)
    log.info("Committed changes for %s / %s", dim, focus)


def revert_changes(allowed_paths: list[str]) -> None:
    log.warning("Reverting changes to allowed paths: %s", allowed_paths)
    for p in allowed_paths:
        git("checkout", "HEAD", "--", p, check=False)
    git("clean", "-fd", check=False)


def write_output(branch: str, dim: str, focus: str, summary: str) -> None:
    title = f"feat({dim}): {focus} - gamedev agent"
    body = (
        f"## Summary\n\n"
        f"Automated gamedev agent improvement.\n\n"
        f"- **Dimension**: `{dim}`\n"
        f"- **Focus**: `{focus}`\n\n"
        f"### Change\n\n{summary}\n\n"
        f"## Verification\n\n"
        f"- `cargo fmt --all -- --check` passed\n"
        f"- `cargo clippy --all-targets -- -D warnings` passed\n"
        f"- `cargo test --all-targets` passed\n\n"
        f"> Generated by the gamedev agent. Review before merging."
    )
    OUTPUT_FILE.write_text(json.dumps({
        "branch": branch,
        "dimension": dim,
        "focus": focus,
        "pr_title": title,
        "pr_body": body,
    }, indent=2), encoding="utf-8")
    log.info("Wrote output to %s", OUTPUT_FILE)


# ---------------------------------------------------------------------------
# Verification (orchestrator-side, after the agent says it is done)
# ---------------------------------------------------------------------------

def run_verify(cmd: list[str]) -> tuple[bool, str]:
    try:
        result = subprocess.run(
            cmd,
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=900,
        )
    except subprocess.TimeoutExpired:
        return False, f"ERROR: {' '.join(cmd)} timed out"
    output = (result.stdout + result.stderr)[:20_000]
    return result.returncode == 0, output


# ---------------------------------------------------------------------------
# Agent loop (Anthropic SDK with proxy support)
# ---------------------------------------------------------------------------

def agent_loop(dim: str, focus: str, scope: str, allowed_paths: list[str]) -> str | None:
    token = os.environ.get("GAMEDEV_AGENT_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if not token:
        log.error("GAMEDEV_AGENT_TOKEN or GITHUB_TOKEN is not set; required for inference")
        return None

    model = os.environ.get("GAMEDEV_AGENT_MODEL", DEFAULT_MODEL)
    base  = os.environ.get("GAMEDEV_AGENT_BASE",  DEFAULT_BASE)
    log.info("Inference: model=%s base=%s", model, base)

    client = Anthropic(auth_token=token, base_url=base) if base != DEFAULT_BASE else Anthropic(auth_token=token)
    dispatch = make_dispatch(allowed_paths)
    tools = tool_schemas()
    
    # Convert OpenAI tool format to Anthropic format
    anthropic_tools = []
    for tool in tools:
        anthropic_tools.append({
            "name": tool["function"]["name"],
            "description": tool["function"]["description"],
            "input_schema": tool["function"]["parameters"],
        })

    messages: list[dict] = [
        {"role": "user", "content": build_task_prompt(dim, focus, scope, allowed_paths)},
    ]

    summary: str | None = None

    for round_num in range(MAX_TOOL_ROUNDS):
        try:
            resp = client.messages.create(
                model=model,
                max_tokens=4096,
                system=SYSTEM_PROMPT,
                messages=messages,
                tools=anthropic_tools,
            )
        except Exception as exc:
            log.error("Inference call failed: %s", exc)
            return None

        # Process response content blocks
        tool_calls = []
        text_response = None
        
        for block in resp.content:
            if block.type == "text":
                text_response = block.text
            elif block.type == "tool_use":
                tool_calls.append(block)
        
        # Append assistant message
        assistant_entry: dict = {
            "role": "assistant",
            "content": []
        }
        
        if text_response:
            assistant_entry["content"].append({"type": "text", "text": text_response})
        
        for tool_call in tool_calls:
            assistant_entry["content"].append({
                "type": "tool_use",
                "id": tool_call.id,
                "name": tool_call.name,
                "input": tool_call.input,
            })
        
        messages.append(assistant_entry)
        
        if not tool_calls:
            if text_response:
                text = text_response.strip()
                if len(text) < 20:
                    log.warning("Agent final message too short (%d chars): %r", len(text), text)
                    return None
                summary = text
                log.info("Agent concluded after %d rounds.", round_num + 1)
            break

        # Execute every requested tool call and append responses.
        tool_results = []
        for tool_call in tool_calls:
            fn_name = tool_call.name
            fn_args = tool_call.input or {}
            
            result_text = None
            log.info("Tool [%d] %s args=%s", round_num + 1, fn_name, list(fn_args.keys()) if isinstance(fn_args, dict) else [])
            
            if fn_name not in dispatch:
                result_text = f"ERROR: unknown tool '{fn_name}'"
            else:
                try:
                    result_text = dispatch[fn_name](fn_args)
                except Exception as exc:
                    result_text = f"ERROR executing {fn_name}: {exc}"
            
            tool_results.append({
                "type": "tool_result",
                "tool_use_id": tool_call.id,
                "content": result_text,
            })
        
        # Append tool results as user message
        if tool_results:
            messages.append({
                "role": "user",
                "content": tool_results,
            })
    else:
        log.error("Agent loop exhausted %d rounds without a final response", MAX_TOOL_ROUNDS)

    return summary


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    state = load_state()

    force_dim   = os.environ.get("FORCE_DIMENSION", "").strip()
    force_focus = os.environ.get("FORCE_FOCUS",     "").strip()

    task: tuple[str, str, str, list[str]] | None
    if force_dim and force_focus:
        # Resolve scope + paths from the catalog
        entries = DIMENSIONS.get(force_dim, [])
        match = next((e for e in entries if e[0] == force_focus), None)
        if match is None:
            log.error("Forced task %s/%s not found in catalog", force_dim, force_focus)
            sys.exit(EXIT_ERROR)
        task = (force_dim, match[0], match[1], match[2])
        log.info("Forced task: %s / %s", force_dim, force_focus)
    else:
        task = pick_next_task(state)
        if task is None:
            log.info("All %d tasks complete.", len(build_task_queue()))
            sys.exit(EXIT_DONE)
        log.info("Selected next task: %s / %s", task[0], task[1])

    dim, focus, scope, allowed_paths = task
    branch = branch_name(dim, focus)

    # Duplicate-branch guard
    existing = git("ls-remote", "--heads", "origin", branch, check=False)
    if existing.stdout.strip():
        log.warning("Branch %s already exists on remote; marking done", branch)
        record_completion(state, dim, focus)
        save_state(state)
        sys.exit(EXIT_SKIP)

    create_branch(branch)

    summary = agent_loop(dim, focus, scope, allowed_paths)

    # Detect any change in the allowed paths
    files_changed = False
    for p in allowed_paths:
        st = git("status", "--porcelain", p, check=False)
        if st.stdout.strip():
            files_changed = True
            break

    if not files_changed:
        log.info("Agent made no file changes; marking task done.")
        record_completion(state, dim, focus, summary)
        save_state(state)
        sys.exit(EXIT_SKIP)

    if summary and summary.upper().startswith("SKIP:"):
        log.info("Agent reported SKIP: %s", summary)
        revert_changes(allowed_paths)
        record_completion(state, dim, focus, summary)
        save_state(state)
        sys.exit(EXIT_SKIP)

    if not summary:
        summary = f"{dim} / {focus}: gamedev agent applied an improvement"

    # Re-verify from the orchestrator (defense in depth in case the agent
    # claimed green without actually re-running).
    for cmd in (
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
        ["cargo", "test", "--all-targets"],
    ):
        log.info("Verifying: %s", " ".join(cmd))
        ok, out = run_verify(cmd)
        if not ok:
            log.error("%s failed:\n%s", " ".join(cmd), out[-4000:])
            revert_changes(allowed_paths)
            record_completion(state, dim, focus)
            save_state(state)
            sys.exit(EXIT_SKIP)

    commit_changes(dim, focus, allowed_paths)
    write_output(branch, dim, focus, summary)
    record_completion(state, dim, focus, summary)
    save_state(state)
    sys.exit(EXIT_COMMITTED)


if __name__ == "__main__":
    main()
