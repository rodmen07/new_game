# SpendSteward Proxy Integration for new_game

**Status**: ✅ **FULLY OPERATIONAL**

This repository now has complete SpendSteward proxy integration. All Claude API calls made by the gamedev agent will be automatically routed through SpendSteward for cost tracking, optimization, and analytics.

## Quick Start (2 minutes)

### 1. Get Your JWT Token

```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

Save the `access_token` value.

### 2. Run the Agent with Proxy

```bash
./run_agent_with_proxy.sh <your-jwt-token>

# Or with Python directly:
python3 run_agent_with_proxy.py --jwt-token <your-jwt-token>
```

### 3. Monitor Costs

In another terminal:
```bash
curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer <your-jwt-token>" | python3 -m json.tool
```

## What's Included

| File | Purpose |
|------|---------|
| `run_agent_with_proxy.py` | Python orchestrator for agent + proxy setup |
| `run_agent_with_proxy.sh` | Shell wrapper (one-liner execution) |
| `AGENT_INTEGRATION.md` | Complete integration guide with examples |
| `PROXY_SETUP_GUIDE.md` | SpendSteward setup walkthrough |
| `SPENDSTEWARD_PROXY.md` | Detailed technical documentation |
| `setup_proxy.py` | Configuration generator |
| `setup_spendsteward_proxy.sh` | Environment setup script |
| `.spendsteward_proxy.json` | Auto-generated proxy config |

## Features Enabled

✅ **Real-time Cost Tracking**
- Every API call logged with input/output tokens
- Cost breakdown by model and provider
- Monthly spending dashboard

✅ **Response Caching** (50%+ savings potential)
- Deterministic requests cached for 1 hour
- Automatic cache hits for identical prompts
- Configurable TTL per user

✅ **Smart Model Routing** (30-50% savings potential)
- Short prompts auto-downgraded to cheaper models
- Configurable rules per provider
- Track savings compared to baseline

✅ **Rate Limiting**
- Monthly token budgets by subscription tier
- Graceful 429 errors when limits exceeded
- Tier upgrade prompts

✅ **Usage Reports**
- CSV export for accounting/billing
- Savings analysis and recommendations
- Performance metrics (latency, cache hit rate)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│         Your Gamedev Agent Code                     │
│     (agents/gamedev/main.py)                        │
└────────────────┬────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────┐
│    run_agent_with_proxy.py/sh                       │
│  (Sets GAMEDEV_AGENT_* environment variables)       │
└────────────────┬────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────┐
│      OpenAI SDK (Anthropic client)                  │
│  (base_url = http://localhost:8000/api/v1/proxy..) │
└────────────────┬────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────┐
│      SpendSteward Proxy                             │
│  • Authenticate with JWT token                      │
│  • Check rate limits                                │
│  • Look for cached responses                        │
│  • Apply model routing rules                        │
│  • Forward to Anthropic API                         │
│  • Log usage and costs                              │
└────────────────┬────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────┐
│      Anthropic Claude API                           │
│  (api.anthropic.com)                                │
└─────────────────────────────────────────────────────┘
```

## Usage Examples

### Basic: Run with Auto-Selected Task

```bash
./run_agent_with_proxy.sh eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Force Specific Dimension/Focus

```bash
./run_agent_with_proxy.sh $JWT_TOKEN --dimension graphics --focus collision-system
```

### Dry-Run Mode (Test Config)

```bash
./run_agent_with_proxy.sh $JWT_TOKEN --dry-run
```

### Custom SpendSteward URL

```bash
./run_agent_with_proxy.sh $JWT_TOKEN --base-url http://192.168.1.100:8000
```

### Manual Environment Setup

```bash
export GAMEDEV_AGENT_BASE="http://localhost:8000/api/v1/proxy/anthropic"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"
export GAMEDEV_AGENT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
python3 agents/gamedev/main.py
```

## Cost Savings

Assuming 100 game dev tasks per month:

| Approach | Input Tokens | Output Tokens | Cost/Month |
|----------|-------------|----------------|-----------|
| Direct API (no optimization) | 500 avg | 1,000 avg | **$1.80** |
| With caching (50% hit rate) | 250 | 500 | **$0.90** |
| With routing (short→cheap model) | 400 | 800 | **$1.08** |
| With routing + caching | 225 | 400 | **$0.63** |

**Pricing**: Claude 3.5 Sonnet ($0.003/1k input, $0.015/1k output)

## Troubleshooting

### Token Expired?

Get a fresh one:
```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

### SpendSteward Not Running?

Start it:
```bash
cd ../SpendSteward/backend
source .venv/bin/activate
python -m uvicorn spendsteward.hosted.app:app --host 127.0.0.1 --port 8000
```

### Agent Fails to Connect?

Check proxy connectivity:
```bash
JWT_TOKEN="your-token"
curl -H "Authorization: Bearer $JWT_TOKEN" \
  http://localhost:8000/healthz
```

Should return: `{"status":"ok","track":"hosted"}`

### Getting 404 Errors?

Verify:
1. Using correct JWT token (not GITHUB_TOKEN)
2. Using correct proxy path: `/api/v1/proxy/anthropic/messages`
3. Anthropic API key registered:
   ```bash
   curl -s http://localhost:8000/api/v1/keys \
     -H "Authorization: Bearer $JWT_TOKEN"
   ```

## Documentation

- **AGENT_INTEGRATION.md** - Complete guide for running agent with proxy
- **PROXY_SETUP_GUIDE.md** - SpendSteward setup walkthrough
- **SPENDSTEWARD_PROXY.md** - Technical architecture and details
- **agents/gamedev/main.py** - Agent implementation

## Test Credentials

For local development:
- Email: `test@example.com`
- Password: `testpassword123`

These are pre-configured in the SpendSteward test setup.

## Next Steps

1. ✅ SpendSteward backend running
2. ✅ Agent proxy integration ready
3. ✅ Authentication configured
4. ⏭️ **Run the agent**: `./run_agent_with_proxy.sh <jwt-token>`
5. ⏭️ Monitor costs: `curl http://localhost:8000/api/v1/usage/summary`
6. ⏭️ Optimize based on dashboard insights

## File Structure

```
new_game/
├── agents/
│   └── gamedev/
│       ├── main.py              (Agent code - no changes needed)
│       ├── prompts.py
│       ├── tasks.py
│       └── tools.py
├── run_agent_with_proxy.py       (🆕 Python orchestrator)
├── run_agent_with_proxy.sh       (🆕 Shell wrapper)
├── AGENT_INTEGRATION.md          (🆕 Integration guide)
├── PROXY_SETUP_GUIDE.md          (Setup walkthrough)
├── SPENDSTEWARD_PROXY.md         (Technical docs)
├── setup_proxy.py                (Config generator)
├── setup_spendsteward_proxy.sh    (Environment setup)
└── .spendsteward_proxy.json       (Generated config)
```

## How It Works

1. You run `run_agent_with_proxy.sh` with your JWT token
2. Script sets environment variables:
   - `GAMEDEV_AGENT_BASE` → SpendSteward proxy URL
   - `GAMEDEV_AGENT_MODEL` → Claude model name
   - `GAMEDEV_AGENT_TOKEN` → JWT token
3. Agent's OpenAI SDK client connects to proxy instead of GitHub Models API
4. Proxy receives request, authenticates, and:
   - Checks rate limits
   - Looks for cached responses
   - Applies model routing
   - Forwards to Anthropic API
   - Logs usage and costs
5. Agent continues normally with the response
6. All costs tracked in SpendSteward dashboard

## Support

Questions? See the docs:
- Integration: `AGENT_INTEGRATION.md`
- Setup: `PROXY_SETUP_GUIDE.md`
- Technical: `SPENDSTEWARD_PROXY.md`

Or check SpendSteward's main repository:
https://github.com/rodmen07/SpendSteward
