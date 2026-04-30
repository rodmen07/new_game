# Agent Integration Guide

Your gamedev agent now has **full SpendSteward proxy integration**. Here's how to use it.

## Quick Start

### 1. Get Your JWT Token

```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

This returns:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "bearer"
}
```

Copy the `access_token` value.

### 2. Run Agent with Proxy (Easy Way)

```bash
# Using the shell wrapper (recommended)
./run_agent_with_proxy.sh eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

# Or using Python directly
python3 run_agent_with_proxy.py --jwt-token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 3. Run Agent with Proxy (Manual Way)

If you prefer to set up environment variables yourself:

```bash
export GAMEDEV_AGENT_BASE="http://localhost:8000/api/v1/proxy/anthropic"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"
export GAMEDEV_AGENT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

python3 agents/gamedev/main.py
```

## Advanced Usage

### Force a Specific Dimension/Focus

```bash
./run_agent_with_proxy.sh <JWT_TOKEN> --dimension graphics --focus collision-system
```

### Dry-Run Mode

Test configuration without starting the agent:

```bash
./run_agent_with_proxy.sh <JWT_TOKEN> --dry-run
```

### Custom SpendSteward URL

If SpendSteward is running on a different machine:

```bash
./run_agent_with_proxy.sh <JWT_TOKEN> --base-url http://192.168.1.100:8000
```

## What Happens When You Run the Agent

1. **Environment Setup**: Agent connects to SpendSteward proxy instead of GitHub Models API
2. **Request Routing**: All Claude API calls route through `http://localhost:8000/api/v1/proxy/anthropic`
3. **Authentication**: JWT token authenticates each request
4. **Cost Tracking**: Every request logged with input/output tokens and cost
5. **Optimization**: 
   - Responses cached for deterministic requests (50%+ savings)
   - Short prompts auto-downgraded to cheaper models (30-50% savings)
6. **Rate Limiting**: Monthly token budgets enforced per tier

## Monitor Costs in Real-Time

While the agent is running, check usage in another terminal:

```bash
JWT_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $JWT_TOKEN" | python3 -m json.tool
```

Shows:
- `totals_by_provider`: Anthropic usage
- `totals_by_model`: Claude Sonnet usage
- `grand_total_cost_usd`: Total spend this month
- `grand_total_requests`: Number of API calls

## Troubleshooting

### SpendSteward Not Running?

Start it:
```bash
cd ../SpendSteward/backend
source .venv/bin/activate
python -m uvicorn spendsteward.hosted.app:app --host 127.0.0.1 --port 8000
```

### JWT Token Expired?

Get a new one:
```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

### Agent Fails to Connect?

Verify proxy is reachable:
```bash
JWT_TOKEN="your-token-here"
curl -H "Authorization: Bearer $JWT_TOKEN" http://localhost:8000/healthz
```

Should return: `{"status":"ok","track":"hosted"}`

### Getting 404 Errors?

Make sure:
1. Using correct JWT token (not GITHUB_TOKEN)
2. Using correct proxy path: `/api/v1/proxy/anthropic`
3. Anthropic API key is registered: 
   ```bash
   curl -s http://localhost:8000/api/v1/keys \
     -H "Authorization: Bearer $JWT_TOKEN"
   ```

## Files

- `run_agent_with_proxy.py` - Python orchestrator (handles all configuration)
- `run_agent_with_proxy.sh` - Shell wrapper (simple one-liner)
- `agents/gamedev/main.py` - Agent code (already supports custom base_url)
- `PROXY_SETUP_GUIDE.md` - Detailed setup documentation

## Architecture

```
Your Agent Code
    ↓
run_agent_with_proxy.py (sets env vars)
    ↓
agents/gamedev/main.py (OpenAI SDK with custom base_url)
    ↓
OpenAI SDK client.chat.completions.create()
    ↓
http://localhost:8000/api/v1/proxy/anthropic/messages
    ↓
SpendSteward Proxy
    ↓
[Rate limit] → [Cache lookup] → [Model routing]
    ↓
http://api.anthropic.com/v1/messages
    ↓
Claude API
    ↓
[Log usage] → [Optimization] → [Return response]
```

## Cost Examples

Assuming:
- 100 game dev tasks per month
- Average: 500 input + 1000 output tokens per task
- Claude 3.5 Sonnet: $0.003/1k input, $0.015/1k output

| Strategy | Cost/Month |
|----------|-----------|
| Direct API (no optimization) | $1.80 |
| With caching (50% hit rate) | $0.90 |
| With routing + caching | $0.60 |

## Questions?

See:
- `PROXY_SETUP_GUIDE.md` - Detailed setup walkthrough
- `SPENDSTEWARD_PROXY.md` - Comprehensive integration guide
- `agents/gamedev/main.py` - Agent implementation
