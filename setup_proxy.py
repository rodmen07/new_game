"""
Setup SpendSteward proxy integration for new_game agent.

This script configures the gamedev agent to use SpendSteward's Claude proxy
instead of GitHub Models, allowing cost tracking and optimization.

Usage:
    python setup_proxy.py --key sk-your-spendsteward-key [--url http://localhost:8000]
"""

import os
import sys
import argparse
import json
from pathlib import Path


def setup_proxy_config(api_key: str, base_url: str = "http://localhost:8000") -> dict:
    """Create proxy configuration for the agent."""
    config = {
        "proxy_enabled": True,
        "spendsteward_base_url": base_url,
        "spendsteward_api_key": api_key,
        "proxy_endpoint": f"{base_url}/v1",
        "claude_model": "claude-3-5-sonnet-20241022",
        "notes": [
            "Cost tracking: All Claude API calls will be tracked in SpendSteward",
            "Optimization: Requests can be automatically routed to cheaper models",
            "Dashboard: View usage and costs at SpendSteward dashboard"
        ]
    }
    return config


def create_proxy_wrapper():
    """Create a wrapper script for running the agent with proxy settings."""
    wrapper_script = '''#!/bin/bash
# Wrapper for new_game gamedev agent with SpendSteward proxy

set -e

# Load proxy configuration
PROXY_CONFIG="${1:-.spendsteward_proxy.json}"

if [ ! -f "$PROXY_CONFIG" ]; then
    echo "Error: $PROXY_CONFIG not found"
    echo "Run: python setup_proxy.py --key <your-api-key>"
    exit 1
fi

# Extract configuration
SPENDSTEWARD_KEY=$(jq -r '.spendsteward_api_key' "$PROXY_CONFIG")
SPENDSTEWARD_BASE=$(jq -r '.spendsteward_base_url' "$PROXY_CONFIG")
CLAUDE_MODEL=$(jq -r '.claude_model' "$PROXY_CONFIG")

echo "🔄 Proxy Configuration:"
echo "  Endpoint: $SPENDSTEWARD_BASE/v1"
echo "  Model: $CLAUDE_MODEL"
echo "  Auth: $(echo $SPENDSTEWARD_KEY | cut -c1-10)..."
echo ""

# Set environment variables
export GAMEDEV_AGENT_BASE="$SPENDSTEWARD_BASE/v1"
export GAMEDEV_AGENT_MODEL="$CLAUDE_MODEL"
export GAMEDEV_AGENT_TOKEN="$SPENDSTEWARD_KEY"

# Note: The agent uses GITHUB_TOKEN for auth; we're using SpendSteward key instead
# The SpendSteward proxy handles authentication transparently
export GITHUB_TOKEN="$SPENDSTEWARD_KEY"

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
        "--key",
        required=True,
        help="SpendSteward API key (sk-...)"
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
    
    # Validate API key format
    if not args.key.startswith("sk-"):
        print("⚠️  Warning: API key should start with 'sk-'")
    
    # Create configuration
    config = setup_proxy_config(args.key, args.url)
    
    # Save configuration
    config_path = Path(args.output)
    with open(config_path, "w") as f:
        json.dump(config, f, indent=2)
    
    print("✅ SpendSteward Proxy Configuration Created")
    print(f"📝 Config file: {config_path}")
    print()
    print("Configuration:")
    print(f"  SpendSteward URL: {args.url}")
    print(f"  Proxy Endpoint:   {args.url}/v1")
    print(f"  Claude Model:     {config['claude_model']}")
    print(f"  API Key:          {args.key[:20]}...")
    print()
    print("Next steps:")
    print("  1. Test the proxy:")
    print(f"     curl -X GET {args.url}/health -H 'Authorization: Bearer {args.key}'")
    print()
    print("  2. Run the agent with proxy:")
    print(f"     source setup_spendsteward_proxy.sh {args.key}")
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
