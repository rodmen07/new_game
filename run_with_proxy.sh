#!/bin/bash
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
