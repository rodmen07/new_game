#!/usr/bin/env python3
"""
Quick reference: How to use SpendSteward Claude proxy with new_game agent

Usage:
    python agents/gamedev/main.py  (use default GitHub Models)
    
    vs.
    
    GAMEDEV_AGENT_BASE="http://localhost:8000/v1" \
    GAMEDEV_AGENT_TOKEN="sk-your-key" \
    GAMEDEV_AGENT_MODEL="claude-3-5-sonnet-20241022" \
    python agents/gamedev/main.py  (use SpendSteward proxy)
"""

print("""
╔════════════════════════════════════════════════════════════════╗
║  SpendSteward Claude Proxy for new_game                       ║
╚════════════════════════════════════════════════════════════════╝

🚀 QUICK START

1. Generate configuration:
   $ python setup_proxy.py --key sk-your-spendsteward-key

2. Run agent with proxy:
   $ source setup_spendsteward_proxy.sh sk-your-spendsteward-key
   $ python agents/gamedev/main.py

3. Monitor costs:
   $ open http://localhost:8000/dashboard


📋 ENVIRONMENT VARIABLES

  GAMEDEV_AGENT_BASE    Base URL of proxy (http://localhost:8000/v1)
  GAMEDEV_AGENT_MODEL   Claude model (claude-3-5-sonnet-20241022)
  GAMEDEV_AGENT_TOKEN   SpendSteward API key (sk-...)
  GITHUB_TOKEN          Used for auth (set to same as GAMEDEV_AGENT_TOKEN)


💰 COSTS

  Claude 3.5 Sonnet: $0.003/1k input, $0.015/1k output
  
  Example task (500 in, 1000 out):
    • Without proxy:      $0.018 per task
    • With caching:       $0.009 (50% hit rate)
    • With routing:       $0.012 (downgrade short prompts)
    • Both optimized:     $0.006


📊 COMMANDS

  # Test SpendSteward is running
  curl http://localhost:8000/health

  # Check usage so far
  curl http://localhost:8000/api/v1/usage/summary \\
    -H "Authorization: Bearer sk-your-key" | jq

  # Export to CSV
  curl http://localhost:8000/api/v1/usage/export \\
    -H "Authorization: Bearer sk-your-key" > usage.csv


🔍 TROUBLESHOOTING

  Connection refused?
    → SpendSteward not running: python -m uvicorn ...
    → Wrong URL: check GAMEDEV_AGENT_BASE
  
  Unauthorized?
    → API key format: should start with "sk-"
    → Check key is active in SpendSteward dashboard
  
  Still going to GitHub Models?
    → Verify env vars are exported: echo $GAMEDEV_AGENT_BASE
    → Restart agent after changing environment
    → Check main.py line 198 reads GAMEDEV_AGENT_BASE


📚 FILES

  setup_spendsteward_proxy.sh    Simple bash setup (loads env vars)
  setup_proxy.py                 Python config generator
  SPENDSTEWARD_PROXY.md          Full documentation
  .spendsteward_proxy.json       Configuration (created by setup_proxy.py)


💡 TIPS

  1. Use temperature=0 for requests to enable caching
  2. Monitor dashboard weekly for usage trends
  3. Check recommended model downgrades in /recommendations
  4. Export usage at month-end for accounting


For detailed setup, see: SPENDSTEWARD_PROXY.md
""")
