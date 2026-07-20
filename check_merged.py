import json
from pathlib import Path

# Check what's in the merged file
merged = json.loads(Path('graphify-out/.graphify_semantic_new.json').read_text(encoding="utf-8"))
print(f"Nodes type: {type(merged.get('nodes'))}")
if merged.get('nodes'):
    print(f"First node: {merged['nodes'][0]}")
    print(f"First node type: {type(merged['nodes'][0])}")