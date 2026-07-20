import json
from graphify.cache import save_semantic_cache
from pathlib import Path

new = json.loads(Path('graphify-out/.graphify_semantic_new.json').read_text(encoding="utf-8"))
uncached = [line for line in Path('graphify-out/.graphify_uncached.txt').read_text(encoding="utf-8").splitlines() if line]
saved = save_semantic_cache(new.get('nodes', []), new.get('edges', []), new.get('hyperedges', []), root='.', allowed_source_files=uncached)
print(f'Cached {saved} files')