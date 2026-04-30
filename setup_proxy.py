"""
Setup SpendSteward proxy integration for new_game agent.

This script configures the gamedev agent to use SpendSteward's Claude proxy
instead of GitHub Models, allowing cost tracking and optimization.

The agent authenticates with SpendSteward using a JWT token obtained from
/auth/jwt/login. The proxy endpoint is /api/v1/proxy/anthropic.

Usage:
    python setup_proxy.py --jwt-token <token> [--url http://localhost:8000]
"""

import os
import sys
import argparse
import json
from pathlib import Path


def setup_proxy_config(jwt_token: str, base_url: str = "http://localhost:8000") -> dict:
    """Create proxy configuration for the agent."""
    config = {
        "proxy_enabled": True,
        "spendsteward_base_url": base_url,
        "jwt_token": jwt_token,
        "proxy_endpoint": f"{base_url}/api/v1/proxy/anthropic",
        "claude_model": "claude-3-5-sonnet-20241022",
        "notes": [
            "Cost tracking: All Claude API calls will be tracked in SpendSteward",
            "Optimization: Requests can be automatically routed to cheaper models",
            "Dashboard: View usage and costs at SpendSteward dashboard",
            "JWT tokens expire after ~1 hour; re-run this script to refresh"
        ]
    }
    return config


def create_proxy_wrapper():
    """Create a wrapper script for running the agent with proxy settings."""
    wrapper_script = '''#!/bin/bash
# Wrapper for new_game gamedev agent with SpendSteward proxy
# This script expects a JWT token (from /auth/jwt/login), not a static API key.
# Use run_agent_with_proxy.sh for the recommended JWT-based workflow.

set -e

# Load proxy configuration
PROXY_CONFIG="${1:-.spendsteward_proxy.json}"

if [ ! -f "$PROXY_CONFIG" ]; then
    echo "Error: $PROXY_CONFIG not found"
    echo "Run: python setup_proxy.py --jwt-token <your-jwt-token>"
    exit 1
fi

# Extract configuration
JWT_TOKEN=$(jq -r '.jwt_token' "$PROXY_CONFIG")
SPENDSTEWARD_BASE=$(jq -r '.spendsteward_base_url' "$PROXY_CONFIG")
CLAUDE_MODEL=$(jq -r '.claude_model' "$PROXY_CONFIG")

echo "🔄 Proxy Configuration:"
echo "  Endpoint: $SPENDSTEWARD_BASE/api/v1/proxy/anthropic"
echo "  Model: $CLAUDE_MODEL"
echo ""

# Set environment variables
export GAMEDEV_AGENT_BASE="$SPENDSTEWARD_BASE/api/v1/proxy/anthropic"
export GAMEDEV_AGENT_MODEL="$CLAUDE_MODEL"
export GAMEDEV_AGENT_TOKEN="$JWT_TOKEN"

echo "✓ Running gamedev agent with SpendSteward proxy..."
echo ""

cd "$(dirname "$0")"
python agents/gamedev/main.py "$@"
'''
    return wrapper_script


def main():
    parser = argparse.ArgumentParser(
        description="Setup SpendSteward proxy for new_game gamedev agent"
    )
    parser.add_argument(
        "--jwt-token",
        required=True,
        help="SpendSteward JWT token (obtained from POST /auth/jwt/login)"
    )
    parser.add_argument(
        "--url",
        default="http://localhost:8000",
        help="SpendSteward base URL (default: http://localhost:8000)"
    )
    parser.add_argument(
        "--output",
        default=".spendsteward_proxy.json",
        help="Output configuration file"
    )

    args = parser.parse_args()

    # Validate JWT token format
    if not args.jwt_token.startswith("eyJ"):
        print("⚠️  Warning: JWT token should start with 'eyJ'")

    # Create configuration
    config = setup_proxy_config(args.jwt_token, args.url)

    # Save configuration
    config_path = Path(args.output)
    with open(config_path, "w") as f:
        json.dump(config, f, indent=2)

    print("✅ SpendSteward Proxy Configuration Created")
    print(f"📝 Config file: {config_path}")
    print()
    print("Configuration:")
    print(f"  SpendSteward URL: {args.url}")
    print(f"  Proxy Endpoint:   {args.url}/api/v1/proxy/anthropic")
    print(f"  Claude Model:     {config['claude_model']}")
    print()
    print("Next steps:")
    print("  1. Test the proxy:")
    print(f"     curl -X GET {args.url}/healthz -H 'Authorization: Bearer <jwt_token>'")
    print()
    print("  2. Run the agent with proxy:")
    print(f"     source setup_spendsteward_proxy.sh <jwt_token>")
    print("     python agents/gamedev/main.py")
    print()
    print("Or use the wrapper:")
    print(f"     bash run_with_proxy.sh {config_path}")
    print()
    print("Monitor costs:")
    print(f"     Visit SpendSteward dashboard at: {args.url}/dashboard")

    # Optionally create wrapper script
    wrapper_path = Path("run_with_proxy.sh")
    if not wrapper_path.exists():
        wrapper_script = create_proxy_wrapper()
        with open(wrapper_path, "w") as f:
            f.write(wrapper_script)
        wrapper_path.chmod(0o755)
        print()
        print(f"📜 Created wrapper script: {wrapper_path}")


if __name__ == "__main__":
    main()
