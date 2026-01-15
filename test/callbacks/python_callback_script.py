#!/usr/bin/env python3

import sys
import json
from datetime import datetime

# Parse arguments
args = sys.argv[1:]
context_index = args.index('--context')
context = json.loads(args[context_index + 1])

# Access metadata (NO PAYLOAD)
print("PYTHON CALLBACK SCRIPT EXECUTED")
print(f"=== Response Metadata ===")
print(f"Status: {context['status_code']}")
print(f"Duration: {context['duration_ms']}ms")
print(f"Method: {context['method']}")
print(f"Path: {context['upstream_path']}")

# Access headers
print("\n=== Headers ===")
for key, value in context['headers'].items():
    print(f"{key}: {value}")

# Custom logic
if context['status_code'] >= 500:
    print("❌ Server error detected!")

if context['duration_ms'] > 1000:
    print("⚠️  Slow response detected!")

print("✅ Callback completed")
