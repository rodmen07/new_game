#!/bin/bash
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
