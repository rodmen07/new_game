#!/bin/bash
# Setup SpendSteward Claude proxy for new_game agent

# Configuration
SPENDSTEWARD_API_KEY="${1:-}"  # Your SpendSteward API key
CLAUDE_API_KEY="${2:-}"         # Your Claude API key (if not using Anthropic OAuth)
SPENDSTEWARD_BASE_URL="${3:-http://localhost:8000}"  # Default to local dev

if [ -z "$SPENDSTEWARD_API_KEY" ]; then
    echo "Usage: $0 <spendsteward_api_key> [claude_api_key] [base_url]"
    echo ""
    echo "Examples:"
    echo "  # Using local SpendSteward dev server"
    echo "  $0 sk-test-12345"
    echo ""
    echo "  # Using remote SpendSteward with custom Claude key"
    echo "  $0 sk-prod-12345 sk-proj-anthropic-key https://api.spendsteward.app"
    echo ""
    echo "Environment variables that will be set:"
    echo "  GAMEDEV_AGENT_BASE    = $SPENDSTEWARD_BASE_URL/v1"
    echo "  GAMEDEV_AGENT_MODEL   = claude-3-5-sonnet-20241022 (standard Claude model)"
    exit 1
fi

# Set environment variables for the agent
export GAMEDEV_AGENT_BASE="$SPENDSTEWARD_BASE_URL/v1"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"

# Use SpendSteward key as auth token (it's already encrypted)
export GAMEDEV_AGENT_TOKEN="$SPENDSTEWARD_API_KEY"

echo "✓ SpendSteward proxy configured:"
echo "  Base URL:  $GAMEDEV_AGENT_BASE"
echo "  Model:     $GAMEDEV_AGENT_MODEL"
echo "  API Key:   $SPENDSTEWARD_API_KEY (first 10 chars shown: ${SPENDSTEWARD_API_KEY:0:10}...)"
echo ""
echo "Ready to run the gamedev agent!"
echo "Run: cd /home/rodmendoza07/Projects/new_game && python agents/gamedev/main.py"
