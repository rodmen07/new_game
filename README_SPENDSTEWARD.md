# SpendSteward Integration Documentation Map

This guide explains the complete SpendSteward proxy integration for the new_game agent.

## 📚 Documentation Structure

### 1. **Start Here: KEY_SETUP.md** ⭐ (First Read)
**Focus:** Understanding the two-key architecture

- What are the two keys?
  - Anthropic API key (one-time, encrypted)
  - JWT token (per-session, temporary)
- Why do we need both?
- One-time setup vs per-session setup
- Request flow diagrams
- Common mistakes to avoid

**When to read:** When you're confused about which key to use

### 2. **QUICKSTART.md** (Next Read)
**Focus:** Getting the agent running in 5 minutes

- Prerequisites checklist
- One-time setup (dependencies)
- Three ways to run the agent
- Monitoring costs
- Troubleshooting

**When to read:** When you want to run the agent ASAP

### 3. **SPENDSTEWARD_INTEGRATION.md** (For Overview)
**Focus:** Complete overview and architecture

- Quick start
- Features enabled
- Architecture diagram
- 8 usage examples
- Cost savings calculations
- File structure

**When to read:** When you want a complete overview

### 4. **AGENT_INTEGRATION.md** (For Details)
**Focus:** Detailed integration walkthrough

- JWT token flow
- Manual environment setup
- Integration examples
- Real-time cost monitoring
- Architecture explanation

**When to read:** When you want detailed integration steps

### 5. **PROXY_SETUP_GUIDE.md** (For SpendSteward)
**Focus:** SpendSteward backend setup

- SpendSteward status
- Working test credentials
- Integration examples
- Python integration code

**When to read:** When setting up SpendSteward backend

### 6. **SPENDSTEWARD_PROXY.md** (For Technical Details)
**Focus:** Technical deep dive

- Comprehensive integration guide
- Cost examples and ROI
- Request flow diagrams
- Advanced usage patterns
- FAQ

**When to read:** When you need technical details

## 🎯 Quick Decision Tree

**"I want to understand the architecture"**
→ KEY_SETUP.md

**"I want to run the agent now"**
→ QUICKSTART.md

**"I want to see everything"**
→ SPENDSTEWARD_INTEGRATION.md

**"I want detailed setup steps"**
→ AGENT_INTEGRATION.md

**"I need to setup SpendSteward backend"**
→ PROXY_SETUP_GUIDE.md

**"I need technical details"**
→ SPENDSTEWARD_PROXY.md

## ✅ Complete Workflow

### One-Time Setup (Already Done ✅)
1. ✅ SpendSteward backend installed and running
2. ✅ Test user created (test@example.com)
3. ✅ Anthropic API key registered and encrypted
4. ✅ Agent dependencies installed (venv + openai)

### Per-Session Workflow (What You Do)
1. Get JWT token (1 command)
2. Run agent (1 command)
3. Monitor costs (optional, 1 command)

See QUICKSTART.md for exact commands.

## 🔑 Key Architecture (30 seconds)

```
┌────────────────────────────┐
│ Anthropic API Key          │
├────────────────────────────┤
│ Your personal API key      │
│ Registered ONCE in         │
│ SpendSteward              │
│ Encrypted at rest          │
│ SpendSteward uses it to    │
│ forward requests           │
└────────────────────────────┘

┌────────────────────────────┐
│ JWT Token                  │
├────────────────────────────┤
│ SpendSteward session token │
│ Get fresh one EACH run     │
│ Used to authenticate       │
│ with SpendSteward proxy    │
│ ➜ THIS is what you pass    │
│   to the agent             │
└────────────────────────────┘
```

## 🚀 Run the Agent (3 commands)

```bash
# 1. Get JWT token
JWT_TOKEN=$(curl -s -X POST http://localhost:8000/auth/jwt/login \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test@example.com&password=testpassword123' | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])")

# 2. Run agent with JWT token
./run_agent_with_proxy.sh $JWT_TOKEN

# 3. (Optional) Monitor costs in another terminal
curl -s http://localhost:8000/api/v1/usage/summary \
  -H "Authorization: Bearer $JWT_TOKEN" | python3 -m json.tool
```

## 📊 File Guide

| File | Purpose | Read When |
|------|---------|-----------|
| KEY_SETUP.md | 🔑 Two-key architecture | Confused about keys |
| QUICKSTART.md | ⚡ Fast 5-min guide | Want to run now |
| SPENDSTEWARD_INTEGRATION.md | 📋 Complete overview | Want full context |
| AGENT_INTEGRATION.md | 🔧 Detailed steps | Want setup details |
| PROXY_SETUP_GUIDE.md | 🛠️ Backend setup | Need SpendSteward help |
| SPENDSTEWARD_PROXY.md | 📚 Technical deep dive | Need internals |
| run_agent_with_proxy.py | 🐍 Python orchestrator | Customizing setup |
| run_agent_with_proxy.sh | 🔨 Shell wrapper | Quick runs |

## 🎓 Learning Path

1. **Beginner** (5 minutes)
   - Read: KEY_SETUP.md + QUICKSTART.md
   - Do: Get JWT token + run agent

2. **Intermediate** (20 minutes)
   - Read: SPENDSTEWARD_INTEGRATION.md
   - Do: Monitor costs + check usage summary

3. **Advanced** (30+ minutes)
   - Read: AGENT_INTEGRATION.md + SPENDSTEWARD_PROXY.md
   - Do: Understand architecture + customize setup

## ✨ Key Features

✅ **Cost Tracking**: Every API call logged with tokens and cost
✅ **Caching**: 50%+ savings on repeated requests  
✅ **Smart Routing**: Cheaper models for simple prompts (30-50% savings)
✅ **Rate Limiting**: Monthly token budgets enforced
✅ **Usage Reports**: CSV export for accounting
✅ **Real-time Monitoring**: Check costs while agent runs

## 🐛 Troubleshooting

| Problem | Solution | See Also |
|---------|----------|----------|
| "ModuleNotFoundError: No module named 'openai'" | `source .venv/bin/activate && pip install openai` | QUICKSTART.md |
| "JWT token required" | Generate fresh token (see QUICKSTART.md) | KEY_SETUP.md |
| "Couldn't connect to localhost:8000" | Start SpendSteward backend | PROXY_SETUP_GUIDE.md |
| "Which key should I use?" | JWT token (not Anthropic key) | KEY_SETUP.md |
| "Where's my Anthropic key?" | Already registered in SpendSteward | KEY_SETUP.md |

## 📞 Questions?

- **Architecture?** → KEY_SETUP.md
- **Getting started?** → QUICKSTART.md
- **Integration?** → AGENT_INTEGRATION.md
- **Technical?** → SPENDSTEWARD_PROXY.md
- **Backend?** → PROXY_SETUP_GUIDE.md

---

**TL;DR:**
1. Read KEY_SETUP.md (understand keys)
2. Read QUICKSTART.md (get commands)
3. Run the agent!

