#!/bin/bash

# SpendSteward Proxy - Virtual Environment Setup
# This script sets up the Python virtual environment with all required dependencies

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  SpendSteward Proxy - Environment Setup                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if venv already exists
if [ -d "$SCRIPT_DIR/.venv" ]; then
    echo "✅ Virtual environment already exists at .venv"
    read -p "   Do you want to recreate it? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "   Skipping venv creation..."
        SKIP_VENV=1
    else
        echo "   Removing existing venv..."
        rm -rf "$SCRIPT_DIR/.venv"
    fi
fi

# Create venv if not skipped
if [ -z "$SKIP_VENV" ]; then
    echo "📦 Creating Python virtual environment..."
    python3 -m venv "$SCRIPT_DIR/.venv"
    echo "   ✓ Created at: $SCRIPT_DIR/.venv"
    echo ""
fi

# Activate venv
echo "🔧 Activating virtual environment..."
source "$SCRIPT_DIR/.venv/bin/activate"
echo "   ✓ Activated"
echo ""

# Upgrade pip
echo "📥 Upgrading pip..."
python3 -m pip install --quiet --upgrade pip
echo "   ✓ pip upgraded"
echo ""

# Install dependencies
echo "📚 Installing dependencies..."
if [ -f "$SCRIPT_DIR/requirements.txt" ]; then
    pip install --quiet -r "$SCRIPT_DIR/requirements.txt"
    echo "   ✓ Installed from requirements.txt"
else
    # Fallback: install commonly needed packages
    pip install --quiet openai anthropic python-dotenv
    echo "   ✓ Installed: openai, anthropic, python-dotenv"
fi
echo ""

# Verify installation
echo "✔️  Verifying installation..."
if python3 -c "import openai" 2>/dev/null; then
    OPENAI_VERSION=$(python3 -c "import openai; print(openai.__version__)")
    echo "   ✓ openai ($OPENAI_VERSION) installed"
else
    echo "   ⚠️  openai not found (may be needed for agent)"
fi
echo ""

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  ✅ Setup Complete                                        ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo ""
echo "1️⃣  Run the agent with proxy:"
echo "   ./run_agent_with_proxy.sh <JWT_TOKEN>"
echo ""
echo "2️⃣  Or use the Python runner directly:"
echo "   python3 run_agent_with_proxy.py --jwt-token <JWT_TOKEN>"
echo ""
echo "3️⃣  To manually activate the venv in your shell:"
echo "   source .venv/bin/activate"
echo ""
echo "4️⃣  To deactivate the venv later:"
echo "   deactivate"
echo ""
