import json
from pathlib import Path

all_nodes = []
all_edges = []
all_hyperedges = []
total_input = 0
total_output = 0

for i in range(7):
    path = Path(f'graphify-out/.graphify_chunk_{i:02d}_extracted.json')
    if path.exists():
        data = json.loads(path.read_text(encoding='utf-8'))
        nodes = data.get('nodes', [])
        edges = data.get('edges', [])
        hyperedges = data.get('hyperedges', [])
        all_nodes.extend(nodes)
        all_edges.extend(edges)
        all_hyperedges.extend(hyperedges)
        total_input += data.get('input_tokens', 0)
        total_output += data.get('output_tokens', 0)
        print(f'Chunk {i}: {len(nodes)} nodes, {len(edges)} edges')
    else:
        print(f'Chunk {i}: MISSING')

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

merged = {
    'nodes': deduped_nodes,
    'edges': deduped_edges,
    'hyperedges': all_hyperedges,
    'input_tokens': total_input,
    'output_tokens': total_output
}
Path('graphify-out/.graphify_semantic_new.json').write_text(json.dumps(merged, indent=2, ensure_ascii=False), encoding='utf-8')
print('Written to graphify-out/.graphify_semantic_new.json')