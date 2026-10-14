#!/usr/bin/env python3
"""Combine reviewed constructor shapes with exact handler provenance; never execute Lua."""
import argparse,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
a=argparse.ArgumentParser();a.add_argument('--check',action='store_true');args=a.parse_args()
features=json.loads((root/'docs/hyprland-implementation-contracts.json').read_text())['features']
shapes=json.loads((root/'scripts/action-shapes.json').read_text());rows=[]
for feature in features:
    if not feature['id'].startswith('api:hl.dsp.'):continue
    name=feature['id'].removeprefix('api:hl.dsp.')
    for pack,source in feature['handler_versions'].items():
        effect='window-or-workspace'
        if name in ['exec_cmd','exec_raw']:effect='external-command'
        elif name in ['exit','reload_config','dpms','force_renderer_reload','force_idle']:effect='session-or-power'
        elif name in ['send_key_state','send_shortcut','pass','release_input_capture']:effect='input-or-capture'
        elif name in ['window.kill','window.signal']:effect='process-termination-or-signal'
        elif name in ['event','global','layout']:effect='external-or-layout'
        elif name=='no_op':effect='inert'
        rows.append(dict(name=name,version={'baseline':'0.56.2','current':'audited-main'}[pack],effect=effect,source=source['source'],source_sha256=source['file_sha256'],variants=shapes[name]))
assert len(rows)==103 and len({(r['name'],r['version']) for r in rows})==103
assert sum(r['version']=='0.56.2' for r in rows)==51
assert sum(r['version']=='audited-main' for r in rows)==52
content=json.dumps(rows,indent=2)+'\n';path=root/'data/actions.json'
if args.check:assert path.read_text()==content,'Action metadata drift; regenerate'
else:path.write_text(content)
print('103 action records match reviewed shapes and pinned handlers')

binding=json.loads((root/'data/bindings.json').read_text())
for pack in binding['packs']:
    version='baseline' if pack['version']=='0.56.2' else 'current'
    feature=next(f for f in features if f['id']=='api:hl.bind')['handler_versions'][version]
    assert pack['source']==feature['source'] and pack['source_sha256']==feature['file_sha256']
    assert set(pack['flags']+pack['description_aliases']+['device']+pack['device_fields'])==set(feature['argument_facts']['other_literal_field_reads'])
print('Binding fields match both pinned source handlers')
