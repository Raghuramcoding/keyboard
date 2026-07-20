import json
from pathlib import Path

uncached = [line for line in Path('graphify-out/.graphify_uncached.txt').read_text(encoding="utf-8").splitlines() if line]
chunk_size = 22
chunks = [uncached[i:i+chunk_size] for i in range(0, len(uncached), chunk_size)]

for i, chunk in enumerate(chunks):
    chunk_path = Path(f'graphify-out/.graphify_chunk_{i:02d}.json')
    chunk_path.write_text(json.dumps(chunk), encoding="utf-8")
    print(f"Chunk {i}: {len(chunk)} files -> {chunk_path}")

print(f"\nTotal chunks: {len(chunks)}")