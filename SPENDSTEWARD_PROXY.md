# SpendSteward Claude Proxy Integration for new_game

This guide shows how to integrate SpendSteward's Claude API proxy with the new_game gamedev agent.

## Overview

The new_game gamedev agent currently uses GitHub Models for Claude API calls. By configuring it to use SpendSteward's proxy, you get:

- ✅ **Cost Tracking**: Every Claude API call is logged and tracked
- ✅ **Optimization**: Automatic model downgrading for short prompts (gpt-4o → gpt-4o-mini equivalent)
- ✅ **Caching**: Deterministic requests (temperature=0) cached to reduce costs
- ✅ **Dashboard**: Real-time visibility into Claude usage and costs
- ✅ **Recommendations**: AI-powered suggestions for further optimization

## Prerequisites

1. **SpendSteward running** (local dev or deployed)
   - Local: `python -m uvicorn spendsteward.hosted.app:app --reload`
   - Remote: Your SpendSteward instance URL

2. **SpendSteward API key** (`sk-...`)
   - Create via dashboard or CLI

3. **Claude API key** (optional if using Anthropic OAuth integration)
   - Encrypted and stored by SpendSteward

## Quick Start (5 minutes)

### Step 1: Create Configuration

```bash
cd /home/rodmendoza07/Projects/new_game

# Generate proxy configuration
python setup_proxy.py --key sk-your-spendsteward-key --url http://localhost:8000
```

This creates `.spendsteward_proxy.json` with your proxy settings.

### Step 2: Run Agent with Proxy

**Option A: Using bash wrapper**
```bash
source setup_spendsteward_proxy.sh sk-your-spendsteward-key
python agents/gamedev/main.py
```

**Option B: Using Python wrapper**
```bash
python setup_proxy.py --key sk-your-spendsteward-key
bash run_with_proxy.sh
```

**Option C: Manual environment variables**
```bash
export GAMEDEV_AGENT_BASE="http://localhost:8000/v1"
export GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022"
export GAMEDEV_AGENT_TOKEN="sk-your-spendsteward-key"
export GITHUB_TOKEN="$GAMEDEV_AGENT_TOKEN"  # Agent uses this for auth

python agents/gamedev/main.py
```

### Step 3: Monitor Costs

Open SpendSteward dashboard:
```
http://localhost:8000/dashboard
```

## How It Works

### Request Flow

```
new_game agent
    ↓
OpenAI SDK (base_url = http://localhost:8000/v1)
    ↓
SpendSteward Proxy (/v1/chat/completions)
    ↓
- Check rate limits
- Extract prompt tokens
- Optional: Route to cheaper model
- Optional: Check cache (if temperature=0)
    ↓
Anthropic Claude API
    ↓
SpendSteward logs usage
    ↓
Response returned to agent
```

### Configuration Options

The agent respects these environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `GAMEDEV_AGENT_BASE` | `https://models.github.ai/inference` | API endpoint (change to SpendSteward) |
| `GAMEDEV_AGENT_MODEL` | `anthropic/claude-sonnet-4-5` | Claude model name |
| `GAMEDEV_AGENT_TOKEN` | (from `GITHUB_TOKEN`) | API authentication token |
| `GITHUB_TOKEN` | (required) | Used for agent auth |

## Claude Model Names

When using SpendSteward proxy, use the actual Anthropic model names:

- `claude-3-5-sonnet-20241022` (recommended)
- `claude-3-opus-20240229` (larger/slower)
- `claude-3-sonnet-20240229` (older version)
- `claude-3-haiku-20240307` (faster/cheaper)

**Note**: GitHub Models uses `anthropic/claude-sonnet-4-5` format, but SpendSteward uses standard Anthropic naming.

## Cost Examples

Assuming Claude Sonnet pricing ($0.003 input / $0.015 output per 1k tokens):

### Scenario 1: Typical Task (500 tokens input, 1000 tokens output)
- Cost: (500 × $0.003 + 1000 × $0.015) / 1000 = **$0.018**
- With caching (50% hit rate): **$0.009**
- With routing: **$0.012** (downgrades short prompts)

### Scenario 2: Monthly Usage (100 tasks)
- Without optimization: 100 × $0.018 = **$1.80**
- With caching enabled: **$0.90**
- With routing + caching: **$0.72**

## Troubleshooting

### "Failed to connect to SpendSteward"
- Ensure SpendSteward is running: `curl http://localhost:8000/health`
- Check base URL is correct (no trailing slash)
- Verify firewall allows access

### "Unauthorized: Invalid API key"
- Ensure API key starts with `sk-`
- Verify key is active (not expired/revoked)
- Check key has "api" permission scope

### "Model not found"
- Use standard Anthropic model names (not GitHub Models format)
- Recommended: `claude-3-5-sonnet-20241022`

### "Requests being sent to GitHub Models"
- Verify `GAMEDEV_AGENT_BASE` is set correctly
- Check environment variable is actually exported: `echo $GAMEDEV_AGENT_BASE`
- Restart agent after changing env vars

### "Rate limit exceeded"
- Check SpendSteward tier limits (free: 250k tokens/month)
- Upgrade tier or request limit increase
- Review usage on dashboard: `/dashboard/usage`

## Advanced Usage

### 1. Enable Response Caching

In `agents/gamedev/main.py`, ensure temperature=0 for deterministic requests:

```python
response = client.messages.create(
    model=model,
    max_tokens=4096,
    temperature=0,  # Enable caching
    messages=messages,
    tools=tools,
)
```

SpendSteward will automatically cache these responses.

### 2. Custom Model Routing Rules

If SpendSteward has routing rules configured, the proxy can automatically downgrade models:

```python
# In agent configuration
# For short/simple prompts, SpendSteward might route to claude-3-haiku-20240307
# instead of claude-3-5-sonnet-20241022
```

### 3. Monitor Usage in Real-Time

```bash
# Terminal 1: Run agent
source setup_spendsteward_proxy.sh sk-your-key
python agents/gamedev/main.py

# Terminal 2: Watch costs
watch -n 1 'curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer sk-your-key" | jq'
```

### 4. Export Costs for Accounting

```bash
# Get usage data
curl http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer sk-your-key" | \
  jq '.monthly_cost_usd'
```

## Migration from GitHub Models

If already using GitHub Models, migration is simple:

1. **Before** (GitHub Models):
   ```python
   client = OpenAI(
       api_key=os.environ["GITHUB_TOKEN"],
       base_url="https://models.github.ai/inference"
   )
   ```

2. **After** (SpendSteward + Claude):
   ```python
   client = OpenAI(
       api_key=os.environ.get("GAMEDEV_AGENT_TOKEN", os.environ["GITHUB_TOKEN"]),
       base_url=os.environ.get("GAMEDEV_AGENT_BASE", "https://models.github.ai/inference")
   )
   ```

   The agent already supports this pattern in `main.py` lines 197-201!

## Integration with SpendSteward Dashboard

Once configured, you'll see new entries on your SpendSteward dashboard:

**Home Tab:**
- New usage bar showing Claude API usage
- Cost breakdown: "Claude: $X.XX"

**Usage Tab:**
- Detailed log of each game dev agent run
- Tokens consumed per task
- Cost per task

**Integrations Tab:**
- Claude integration (Anthropic OAuth) status
- API key management

## Next Steps

1. **Test with a small task**: Run agent on a single game dev task to verify setup
2. **Review costs**: Check SpendSteward dashboard to see actual costs
3. **Enable caching**: Ensure temperature=0 requests for maximum savings
4. **Monitor regularly**: Check `/dashboard/usage` weekly for trends

## Support

For issues:

1. Check SpendSteward health: `curl http://localhost:8000/health`
2. Review agent logs: Output from `python agents/gamedev/main.py`
3. Check SpendSteward logs: Console output from FastAPI server
4. Verify configuration: `cat .spendsteward_proxy.json`

## FAQ

**Q: Will this affect game development output quality?**  
A: No. You're using the same Claude models, just routed through SpendSteward for cost tracking.

**Q: Can I still use GitHub Models?**  
A: Yes! Just don't set `GAMEDEV_AGENT_BASE` and `GAMEDEV_AGENT_TOKEN`, and the agent will use GitHub Models defaults.

**Q: What if SpendSteward goes down?**  
A: The agent will fail. Consider running SpendSteward in a docker container for reliability.

**Q: Can I use this with other AI tools?**  
A: Yes, SpendSteward proxy works with any tool using the OpenAI SDK.
