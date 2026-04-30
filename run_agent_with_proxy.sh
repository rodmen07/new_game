#!/bin/bash
# Wrapper script to run gamedev agent with SpendSteward proxy
# Usage: ./run_agent_with_proxy.sh [JWT_TOKEN] [OPTIONS]

set -e

# Get JWT token from argument or environment
JWT_TOKEN="${1:-${SPENDSTEWARD_JWT:-}}"

if [ -z "$JWT_TOKEN" ]; then
    echo "❌ ERROR: JWT token required"
    echo ""
    echo "Usage: $0 <JWT_TOKEN> [OPTIONS]"
    echo ""
    echo "Get a JWT token:"
    echo '  curl -X POST http://localhost:8000/auth/jwt/login \'
    echo '    -H "Content-Type: application/x-www-form-urlencoded" \'
    echo '    -d "username=test@example.com&password=testpassword123"'
    echo ""
    echo "Example:"
    echo "  $0 eyJhbGciOiJIUzI1NiI... --dimension graphics"
    exit 1
fi

# Shift to get remaining arguments
shift

# Activate virtual environment
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$SCRIPT_DIR/.venv/bin/activate" ]; then
    echo "❌ ERROR: Virtual environment not found at $SCRIPT_DIR/.venv"
    echo ""
    echo "Run setup first:"
    echo "  python3 -m venv .venv"
    echo "  source .venv/bin/activate"
    echo "  pip install anthropic"
    exit 1
fi

source "$SCRIPT_DIR/.venv/bin/activate"

# Run the Python wrapper with the token
exec python3 "$SCRIPT_DIR/run_agent_with_proxy.py" --jwt-token "$JWT_TOKEN" "$@"
