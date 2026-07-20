import json
from pathlib import Path

uncached = [line for line in Path('graphify-out/.graphify_uncached.txt').read_text(encoding="utf-8").splitlines() if line]
print(f"Total uncached files: {len(uncached)}")
for f in uncached[:20]:
    print(f"  {f}")
print("...")