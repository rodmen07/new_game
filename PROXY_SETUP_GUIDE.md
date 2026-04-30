# Using SpendSteward Proxy with new_game

## Status ✅

The SpendSteward backend is running on **localhost:8000** and your Anthropic API key has been registered.

## Quick Start

### Step 1: Create a SpendSteward Account (Done)
- Backend running: http://localhost:8000/healthz ✅
- Test user registered: test@example.com ✅
- Anthropic API key added ✅

### Step 2: Get Your JWT Token
Your test credentials are:
- Email: `test@example.com`
- Password: `testpassword123`

### Step 3: Run the Gamedev Agent with Proxy

The proxy works via standard HTTP requests with JWT authentication. To use it with your gamedev agent:

```bash
cd /home/rodmendoza07/Projects/new_game

# Option A: Direct curl test
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkOWJiNjEzNS04OWI0LTQwY2QtYWNmZC1hYWJkOWE1MDYyYjYiLCJhdWQiOlsiZmFzdGFwaS11c2VyczphdXRoIl0sImV4cCI6MTc3NzU1OTY0OH0.Lalw_GfSoP0pHDwjSFPcawCj2Rdd6wi3FIjUhx7ApCk"

curl -X POST http://localhost:8000/api/v1/proxy/anthropic/messages \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Step 4: Monitor Usage

View your API usage on the SpendSteward dashboard:
```
http://localhost:8000/api/v1/usage/summary
```

With your JWT token:
```bash
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkOWJiNjEzNS04OWI0LTQwY2QtYWNmZC1hYWJkOWE1MDYyYjYiLCJhdWQiOlsiZmFzdGFwaS11c2VyczphdXRoIl0sImV4cCI6MTc3NzU1OTY0OH0.Lalw_GfSoP0pHDwjSFPcawCj2Rdd6wi3FIjUhx7ApCk"

curl -H "Authorization: Bearer $TOKEN" http://localhost:8000/api/v1/usage/summary | python3 -m json.tool
```

## Integration with Your Agent

To make your gamedev agent route through SpendSteward:

### Option 1: Python Integration
```python
import httpx
import json

BASE_URL = "http://localhost:8000"
AUTH_TOKEN = "your-jwt-token"

async def call_claude_via_proxy(prompt: str):
    headers = {
        "Authorization": f"Bearer {AUTH_TOKEN}",
        "Content-Type": "application/json"
    }
    
    body = {
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 1000,
        "messages": [{"role": "user", "content": prompt}]
    }
    
    async with httpx.AsyncClient() as client:
        response = await client.post(
            f"{BASE_URL}/api/v1/proxy/anthropic/messages",
            headers=headers,
            json=body
        )
        return response.json()
```

### Option 2: Modify Agent's Anthropic Client
The gamedev agent uses the OpenAI SDK. To route through SpendSteward, modify the client initialization:

```python
from anthropic import Anthropic

client = Anthropic(
    api_key="jwt-token",
    base_url="http://localhost:8000/api/v1/proxy/anthropic"
)
```

## Cost Tracking

All requests through the proxy are automatically logged. Check real-time stats:

```bash
curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

Response shows:
- `totals_by_provider`: Anthropic usage
- `totals_by_model`: Claude Sonnet usage
- `grand_total_cost_usd`: Total spend this month
- `grand_total_requests`: Number of API calls

## Features Enabled

✅ **Cost Tracking**: Every API call logged with input/output tokens and cost
✅ **Response Caching**: Deterministic requests (temp=0) cached for 1 hour
✅ **Model Routing**: Short prompts automatically downgraded to cheaper models
✅ **Rate Limiting**: Monthly token limits enforced per tier
✅ **Usage Export**: Export usage data for accounting/billing

## Troubleshooting

**Q: Getting 404 errors?**
- Make sure to use `/api/v1/proxy/anthropic/messages` (not `/v1/messages`)
- Verify JWT token is valid and not expired
- Check that Anthropic key is registered: `curl -H "Authorization: Bearer $TOKEN" http://localhost:8000/api/v1/keys`

**Q: Need a new JWT token?**
```bash
curl -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123'
```

**Q: Backend not running?**
```bash
cd /home/rodmendoza07/Projects/SpendSteward/backend
source .venv/bin/activate
python -m uvicorn spendsteward.hosted.app:app --host 127.0.0.1 --port 8000
```

## Next Steps

1. Modify your gamedev agent to route requests through SpendSteward
2. Run your agent and watch usage accumulate
3. Check the dashboard for savings opportunities
4. Optimize expensive requests based on recommendations

Questions? See `/home/rodmendoza07/Projects/new_game/SPENDSTEWARD_PROXY.md`
