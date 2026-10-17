#!/usr/bin/env python3
"""Validate generated metadata without evaluating source expressions."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
rows = json.loads((root / 'data/scalars.json').read_text())
assert len(rows) == 755
assert len({(r['version'], r['id']) for r in rows}) == len(rows)
assert sum(r['version'] == '0.56.2' for r in rows) == 353
assert sum(r['version'] == 'audited-main' for r in rows) == 402
kinds = {'Bool', 'Int', 'Float', 'String', 'Color', 'Gradient', 'Vec2', 'CssGap', 'FontWeight'}
for row in rows:
    assert row['kind'] in kinds
    assert row['lua_key'] == row['id'].replace(':', '.').replace('-', '_')
    assert row['source'].startswith('https://github.com/hyprwm/Hyprland/blob/')
    assert set(row['bounds']) <= {'min', 'max'}
    assert all(isinstance(v, (int, float)) for v in row['bounds'].values())
features = json.loads((root / 'data/features.json').read_text())
assert len(features) == 1453
assert len({r['id'] for r in features}) == len(features)
base=json.loads((root/'data/devices.json').read_text())
added=json.loads((root/'data/device-additions.json').read_text())
assert len(added)==8
assert len({(r['version'],r['id']) for r in base+added})==94
for version in ['0.56.2','audited-main']:
    assert sum(r['version']==version for r in base+added)==47
supplement=json.loads((root/'data/source-additions.json').read_text())
assert len(supplement['requirements'])==5
print('Validated 755 versioned scalar records, 1453 original feature contracts, 5 additional requirements and 47 device fields per snapshot')
