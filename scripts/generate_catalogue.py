#!/usr/bin/env python3
"""Regenerate application metadata from the checked-in, source-pinned audit register."""
import argparse
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[1]
args = argparse.ArgumentParser()
args.add_argument('--check', action='store_true')
check = args.parse_args().check
inventory = json.loads((root / 'docs/hyprland-feature-inventory.json').read_text())
contracts = json.loads((root / 'docs/hyprland-implementation-contracts.json').read_text())
rows = []
versions = [('0.56.2', inventory['scalar_settings']),
            ('audited-main', inventory['current_source_comparison']['scalar_settings'])]
for version, items in versions:
    for setting in items:
        bounds = {}
        for key in ('min', 'max'):
            match = re.search(r'\.' + key + r'\s*=\s*(-?\d+(?:\.\d+)?)(?=\s*[,}])',
                              setting.get('metadata_cpp', ''))
            if match:
                bounds[key] = float(match[1])
        rows.append(dict(id=setting['id'], version=version, kind=setting['type'],
                         lua_key=setting['id'].replace(':', '.').replace('-', '_'),
                         default_expression=setting['default_cpp'], bounds=bounds,
                         source=setting['source']))
keys = {'id', 'kind', 'engine', 'version_membership', 'editor', 'codec',
        'runtime_observation', 'activation', 'preview', 'effect', 'source_versions'}
features = [{k: v for k, v in feature.items() if k in keys}
            for feature in contracts['features']]
def structured(engine, prefix, expected):
    result = []
    for feature in features:
        if feature['engine'] != engine or feature['kind'] != 'structured_field':
            continue
        for source_name, source in feature['source_versions'].items():
            version = {'baseline': '0.56.2', 'current': 'audited-main'}[source_name]
            constructor = source['constructor_cpp']
            parts = constructor.split(', ') if source['type'] in ('Int', 'Float') else [constructor]
            bounds = {'min': None, 'max': None}
            for key, value in zip(('min', 'max'), parts[1:3]):
                bounds[key] = float(value.rstrip('fF'))
            result.append(dict(id=prefix + ':' + source['name'], version=version,
                kind=source['type'], lua_key=source['name'], default_expression=parts[0],
                bounds=bounds, source=source['source']))
    assert len(result) == expected * 2 and len({m['id'] for m in result}) == expected
    return result
monitors = structured('MONITOR', 'monitor', 22)
devices = structured('DEVICE', 'device', 43)
workspaces = structured('WORKSPACE', 'workspace', 15)
for name, data in [('scalars.json', rows), ('features.json', features), ('monitors.json', monitors), ('devices.json', devices), ('workspaces.json', workspaces)]:
    path = root / 'data' / name
    content = json.dumps(data, indent=2) + '\n'
    if check:
        assert path.read_text() == content, f'{name} differs from the pinned audit; regenerate it'
    else:
        path.write_text(content)
print('Catalogue matches pinned audit' if check else 'Regenerated catalogue')
