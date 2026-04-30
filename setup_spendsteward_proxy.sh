#!/bin/bash
# Setup SpendSteward Claude proxy for new_game agent

# Configuration
JWT_TOKEN="${1:-}"              # Your SpendSteward JWT token (from /auth/jwt/login)
SPENDSTEWARD_BASE_URL="${2:-http://localhost:8000}"  # Default to local dev

if [ -z "$JWT_TOKEN" ]; then
    echo "Usage: $0 <jwt_token> [base_url]"
    echo ""
    echo "Get a JWT token first:"
    echo '  curl -s -X POST http://localhost:8000/auth/jwt/login \'
    echo '    -H "Content-Type: application/x-www-form-urlencoded" \'
    echo '    -d "username=<email>&password=<password>"'
    echo ""
    echo "Examples:"
    echo "  # Using local SpendSteward dev server"
    echo "  $0 eyJhbGciOiJIUzI1NiI..."
    echo ""
    echo "  # Using remote SpendSteward instance"
    echo "  $0 eyJhbGciOiJIUzI1NiI... https://api.spendsteward.app"
    echo ""
    echo "Environment variables that will be set:"
    echo "  GAMEDEV_AGENT_BASE    = <base_url>/api/v1/proxy/anthropic"
    echo "  GAMEDEV_AGENT_MODEL   = claude-3-5-sonnet-20241022 (standard Claude model)"
    exit 1
fi

# Set environment variables for the agent
export GAMEDEV_AGENT_BASE="$SPENDSTEWARD_BASE_URL/api/v1/proxy/anthropic"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"

# Use JWT token as auth token
export GAMEDEV_AGENT_TOKEN="$JWT_TOKEN"

echo "✓ SpendSteward proxy configured:"
echo "  Base URL:  $GAMEDEV_AGENT_BASE"
echo "  Model:     $GAMEDEV_AGENT_MODEL"
echo ""
echo "Ready to run the gamedev agent!"
echo "Run: python agents/gamedev/main.py"
