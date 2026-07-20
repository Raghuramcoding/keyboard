import json
from pathlib import Path

chunks = []
for i in range(7):
    path = Path(f'graphify-out/.graphify_chunk_{i:02d}_extracted.json')
    if path.exists():
        data = json.loads(path.read_text(encoding='utf-8'))
        chunks.append(data)
        print(f'Chunk {i}: {len(data.get("nodes", []))} nodes, {len(data.get("edges", []))} edges, {len(data.get("hyperedges", []))} hyperedges')
    else:
        print(f'Chunk {i}: MISSING')

# Merge all
all_nodes = []
all_edges = []
all_hyperedges = []
total_input = 0
total_output = 0

for c in chunks:
    all_nodes.extend(c.get('nodes', []))
    all_edges.extend(c.get('edges', []))
    all_hyperedges.extend(c.get('hyperedges', []))
    total_input += c.get('input_tokens', 0)
    total_output += c.get('output_tokens', 0)

# Deduplicate nodes by id
seen = set()
deduped_nodes = []
for n in all_nodes:
    if n['id'] not in seen:
        seen.add(n['id'])
        deduped_nodes.append(n)

# Deduplicate edges by (source, target, type)
seen_edges = set()
deduped_edges = []
for e in all_edges:
    key = (e['source'], e['target'], e.get('type', ''))
    if key not in seen_edges:
        seen_edges.add(key)
        deduped_edges.append(e)

print(f'\nTotal before dedup: {len(all_nodes)} nodes, {len(all_edges)} edges')
print(f'Total after dedup: {len(deduped_nodes)} nodes, {len(deduped_edges)} edges')

# Write merged
merged = {
    'nodes': deduped_nodes,
    'edges': deduped_edges,
    'hyperedges': all_hyperedges,
    'input_tokens': total_input,
    'output_tokens': total_output
}
Path('graphify-out/.graphify_semantic_new.json').write_text(json.dumps(merged, indent=2, ensure_ascii=False), encoding='utf-8')
print('Written to graphify-out/.graphify_semantic_new.json')