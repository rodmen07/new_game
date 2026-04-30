#!/usr/bin/env python3
"""
Run the gamedev agent with SpendSteward proxy integration.

This script configures the agent to route all Claude API calls through
SpendSteward for cost tracking, caching, and optimization.

Usage:
    python3 run_agent_with_proxy.py [options]

Options:
    --jwt-token TOKEN        JWT token for SpendSteward auth (or use env var SPENDSTEWARD_JWT)
    --base-url URL          SpendSteward base URL (default: http://localhost:8000)
    --model MODEL           Claude model to use (default: claude-3-5-sonnet-20241022)
    --dimension DIM         Force a dimension (optional)
    --focus FOCUS           Force a focus (optional, requires --dimension)
    --dry-run              Show environment variables without running agent
    --help                 Show this help message
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(
        description="Run gamedev agent with SpendSteward proxy",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    parser.add_argument(
        "--jwt-token",
        help="JWT token for SpendSteward authentication"
    )
    parser.add_argument(
        "--base-url",
        default="http://localhost:8000",
        help="SpendSteward base URL (default: http://localhost:8000)"
    )
    parser.add_argument(
        "--model",
        default="claude-3-5-sonnet-20241022",
        help="Claude model to use (default: claude-3-5-sonnet-20241022)"
    )
    parser.add_argument(
        "--dimension",
        help="Force a specific dimension"
    )
    parser.add_argument(
        "--focus",
        help="Force a specific focus (requires --dimension)"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show environment variables without running agent"
    )
    
    args = parser.parse_args()
    
    # Get JWT token from args or environment
    jwt_token = args.jwt_token or os.environ.get("SPENDSTEWARD_JWT")
    
    if not jwt_token:
        print("❌ ERROR: JWT token required")
        print("   Provide via --jwt-token or SPENDSTEWARD_JWT environment variable")
        print()
        print("Get a JWT token:")
        print('  curl -X POST http://localhost:8000/auth/jwt/login \\')
        print('    -H "Content-Type: application/x-www-form-urlencoded" \\')
        print('    -d "username=test@example.com&password=testpassword123"')
        sys.exit(1)
    
    # Prepare environment
    env = os.environ.copy()
    
    # SpendSteward proxy configuration
    env["GAMEDEV_AGENT_BASE"] = f"{args.base_url}/api/v1/proxy/anthropic"
    env["GAMEDEV_AGENT_TOKEN"] = jwt_token
    env["GAMEDEV_AGENT_MODEL"] = args.model
    
    # Optional: override GitHub token (if using proxy)
    # Note: SpendSteward uses the JWT token as the API key
    if "GITHUB_TOKEN" not in env:
        env["GITHUB_TOKEN"] = jwt_token
    
    # Optional: dimension/focus forcing
    if args.dimension:
        env["FORCE_DIMENSION"] = args.dimension
        if args.focus:
            env["FORCE_FOCUS"] = args.focus
    
    # Show configuration
    print("╔════════════════════════════════════════════════════════════╗")
    print("║  Gamedev Agent with SpendSteward Proxy Configuration      ║")
    print("╚════════════════════════════════════════════════════════════╝")
    print()
    print("📊 SpendSteward Configuration:")
    print(f"  Base URL:   {args.base_url}")
    print(f"  Proxy URL:  {args.base_url}/api/v1/proxy/anthropic")
    print(f"  Model:      {args.model}")
    print(f"  Auth:       JWT Token (first 20 chars: {jwt_token[:20]}...)")
    print()
    print("🎯 Agent Configuration:")
    if args.dimension:
        print(f"  Dimension:  {args.dimension}")
        if args.focus:
            print(f"  Focus:      {args.focus}")
    else:
        print("  Dimension:  (auto-select)")
    print()
    print("✨ Features:")
    print("  ✓ Cost tracking for all Claude API calls")
    print("  ✓ Response caching (50%+ savings potential)")
    print("  ✓ Model routing (30-50% optimization)")
    print("  ✓ Real-time dashboard monitoring")
    print()
    
    if args.dry_run:
        print("📝 Environment Variables (dry-run mode):")
        print()
        for key in ["GAMEDEV_AGENT_BASE", "GAMEDEV_AGENT_MODEL", "GAMEDEV_AGENT_TOKEN"]:
            if key in env:
                val = env[key]
                if "TOKEN" in key or "KEY" in key:
                    val = val[:20] + "..." if len(val) > 20 else val
                print(f"  {key}={val}")
        print()
        print("Run without --dry-run to execute the agent.")
        return 0
    
    print("🚀 Starting agent...")
    print()
    
    # Run agent
    agent_script = Path(__file__).parent / "agents" / "gamedev" / "main.py"
    if not agent_script.exists():
        print(f"❌ ERROR: Agent script not found: {agent_script}")
        sys.exit(1)
    
    # Use venv Python if available
    venv_python = Path(__file__).parent / ".venv" / "bin" / "python3"
    python_exe = str(venv_python) if venv_python.exists() else "python3"
    
    try:
        result = subprocess.run(
            [python_exe, str(agent_script)],
            env=env,
            cwd=Path(__file__).parent
        )
        return result.returncode
    except KeyboardInterrupt:
        print("\n⚠️  Agent interrupted by user")
        return 130
    except Exception as e:
        print(f"❌ ERROR: Failed to run agent: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
