# Quick Start Guide - Run Agent with SpendSteward Proxy

## Prerequisites

✅ SpendSteward backend running on `http://localhost:8000`
✅ Anthropic API key provided (stored in `.spendsteward_proxy.json`)
✅ Test user credentials (test@example.com / testpassword123)

## Setup (One Time)

### 1. Set Up Virtual Environment

```bash
cd /home/rodmendoza07/Projects/new_game
./setup_venv.sh
```

This will:
- Create a Python virtual environment in `.venv/`
- Install required dependencies (openai, anthropic, python-dotenv)
- Verify the installation

**Manual setup (if you prefer):**
```bash
python3 -m venv .venv
source .venv/bin/activate
pip install openai anthropic python-dotenv
```

### 2. Verify SpendSteward is Running

```bash
curl http://localhost:8000/healthz
# Should return: {"status":"ok","track":"hosted"}
```

## Run the Agent

### ⚠️ Important: Which Key to Use?

- **Anthropic API Key** (`sk-ant-api03-...`): Already registered in SpendSteward ✅
- **JWT Token**: Generate fresh token for each session (see below)

### Option A: Using the Shell Wrapper (Easiest)

```bash
# Step 1: Get your JWT token (NOT the Anthropic key!)
JWT_TOKEN=$(curl -s -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123' | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])")

# Step 2: Run the agent with JWT token
./run_agent_with_proxy.sh $JWT_TOKEN
```

### Option B: Using Python Directly

```bash
source .venv/bin/activate
python3 run_agent_with_proxy.py --jwt-token $JWT_TOKEN
```

### Option C: Manual Environment Setup

```bash
source .venv/bin/activate

# Use JWT token (not Anthropic API key!)
export GAMEDEV_AGENT_BASE="http://localhost:8000/api/v1/proxy/anthropic"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"
export GAMEDEV_AGENT_TOKEN="$JWT_TOKEN"
export GITHUB_TOKEN="$JWT_TOKEN"

python3 agents/gamedev/main.py
```

## Monitor Costs

While the agent is running, in another terminal:

```bash
JWT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $JWT_TOKEN" | python3 -m json.tool
```

## Troubleshooting

### "ModuleNotFoundError: No module named 'openai'"

Make sure you've activated the virtual environment and installed dependencies:
```bash
source .venv/bin/activate
pip install openai
```

### "JWT token required"

Generate a new JWT token:
```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

### "Couldn't connect to localhost:8000"

Make sure SpendSteward backend is running:
```bash
cd ../SpendSteward/backend
source .venv/bin/activate
python -m uvicorn spendsteward.hosted.app:app --host 127.0.0.1 --port 8000
```

### "GET http://localhost:8000/api/v1/usage/summary ... 404 Not Found"

Make sure:
1. Using JWT token (not GITHUB_TOKEN)
2. Token hasn't expired
3. Backend is running

## What Happens Next

1. Agent gets JWT token from environment or args
2. Agent starts with SpendSteward proxy base URL
3. All Claude API calls route through proxy
4. Each call is:
   - Authenticated with JWT
   - Checked against rate limits
   - Searched in cache
   - Optionally routed to cheaper model
   - Forwarded to Anthropic Claude API
   - Logged with cost and tokens
5. Response returned to agent
6. Agent continues with tools and iterations

## Viewing Results

### Usage Summary
```bash
curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $JWT_TOKEN" | python3 -m json.tool
```

### Cost Breakdown
```bash
curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $JWT_TOKEN" | \
  python3 -c "import sys, json; d=json.load(sys.stdin); print(f'Total Cost: \${d[\"grand_total_cost_usd\"]:.4f}'); print(f'Requests: {d[\"grand_total_requests\"]}')"
```

## Documentation

- **SPENDSTEWARD_INTEGRATION.md** - Full overview and architecture
- **AGENT_INTEGRATION.md** - Detailed integration guide
- **PROXY_SETUP_GUIDE.md** - SpendSteward setup walkthrough
- **SPENDSTEWARD_PROXY.md** - Technical deep dive

## Files Provided

- `run_agent_with_proxy.sh` - Shell wrapper (recommended for quick runs)
- `run_agent_with_proxy.py` - Python orchestrator (more control)
- `.venv/` - Virtual environment with dependencies
- `.spendsteward_proxy.json` - Proxy configuration

## Key Features Enabled

✅ **Cost Tracking**: Every API call logged with tokens and cost
✅ **Caching**: 50%+ savings on repeated requests
✅ **Smart Routing**: Cheaper models for simple prompts (30-50% savings)
✅ **Real-time Monitoring**: Check costs while agent runs
✅ **Rate Limiting**: Monthly token budgets enforced

## Next Steps

1. ✅ Dependencies installed
2. ⏭️ Get JWT token (see above)
3. ⏭️ Run agent: `./run_agent_with_proxy.sh <token>`
4. ⏭️ Monitor costs: `curl http://localhost:8000/api/v1/usage/summary`
5. ⏭️ Check dashboard for optimization recommendations

Questions? See the full documentation in SPENDSTEWARD_INTEGRATION.md
