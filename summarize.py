import json
from graphify.detect import detect
from pathlib import Path
result = detect(Path('.'))
print(f'total_files: {result["total_files"]}')
print(f'total_words: {result["total_words"]}')
for k, v in result['files'].items():
    print(f'{k}: {len(v)} files')
print(f'skipped_sensitive: {len(result.get("skipped_sensitive", []))} files')