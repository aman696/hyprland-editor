#!/usr/bin/env python3
"""Generate data-only registration semantics; never execute C++ or read user config."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

ROOT=Path(__file__).resolve().parents[1]
OUTPUT=ROOT/'data/scalar-semantics.json'
FINDINGS=ROOT/'docs/registration-scope-findings.json'
DEVICE_ADDITIONS=ROOT/'data/device-additions.json'
SCOPE_ADDITIONS=ROOT/'data/source-additions.json'
VERSIONS={'baseline':'0.56.2','current':'audited-main'}


def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def raw(text):return bytes.fromhex(text)
def decoded(text):return raw(text).decode('utf-8')
def number(bits):return struct.unpack('!f',bytes.fromhex(bits))[0]
def names(values):return None if values is None else [{'name':list(raw(name)),'value':value} for name,value in values]


def source_choices(metadata):
    match=re.search(r'strChoice\(\{(.*?)\}\)',metadata,re.S)
    if not match:return None
    body=match[1]
    strings=re.findall(r'"(?:\\.|[^"\\])*"',body)
    if re.sub(r'"(?:\\.|[^"\\])*"|[\s,]','',body):raise ValueError('Review nonliteral string choices')
    return [json.loads(text).encode('utf-8') for text in strings]


def native_value(kind,value):
    if value is None:return None
    if kind=='Float':return {'kind':'float_bits','value':int(value,16)}
    if kind=='String':return {'kind':'bytes','value':list(raw(value))}
    if kind=='Bool':return {'kind':'bool','value':value}
    if kind=='Int':return {'kind':'int','value':value}
    return None


def codec(lua,choices,metadata):
    kind=lua['kind']
    if kind=='Bool':return {'kind':'bool'}
    if kind=='Int':return {'kind':'int','minimum':lua['minimum'],'maximum':lua['maximum'],'names':names(lua['names'])}
    if kind=='Float':return {'kind':'float','minimum':None if lua['minimum'] is None else number(lua['minimum']),'maximum':None if lua['maximum'] is None else number(lua['maximum'])}
    if kind=='String':
        validator=None
        if lua['has_validator']:
            if choices is None:raise ValueError('Unclassified active native string validator')
            for candidate,accepted in lua['validator_cases']:
                if (raw(candidate) in choices)!=accepted:raise ValueError('Native validator differs from literal closed choices')
            validator={'choices':[list(value) for value in choices]}
        return {'kind':'string','validator':validator}
    if kind=='unmodeled_complex':return None
    if kind in ['Color','Gradient','FontWeight']:return {'kind':{'Color':'color','Gradient':'gradient','FontWeight':'font_weight'}[kind]}
    if kind=='CssGap':return {'kind':'css_gap','minimum':lua['minimum'],'maximum':lua['maximum']}
    if kind=='Vec2':
        bounds=None
        if lua['has_validator']:
            match=re.search(r'vec2Range\(([^)]*)\)',metadata)
            if match is None:raise ValueError('Review active native vector validator')
            args=[float(v.strip()) for v in match[1].split(',')]
            if len(args)!=4:raise ValueError('Review vector bounds arity')
            bounds={'minimum':args[:2],'maximum':args[2:]}
            for x,y,accepted in lua['validator_cases']:
                values=[number(f'{x:08x}'),number(f'{y:08x}')]
                modeled=not any(values[i]<args[i] or values[i]>args[i+2] for i in range(2))
                if modeled!=accepted:raise ValueError('Original native vector validator differs from metadata')
        return {'kind':'vec2','validator':bounds}
    raise ValueError('Unclassified Lua constructor kind')


def entry(role,version,key,kind,lua,source=None):
    choices=source_choices(source.get('metadata_cpp','')) if source else None
    return {'role':role,'version':version,'id':key,'lua_key':key,'kind':kind,
            'native_default':native_value(kind,lua.get('native_default')),'codec':codec(lua,choices,source.get('metadata_cpp','') if source else '')}


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    fixture_path=ROOT/'data/scalar-registration-fixtures.json'
    fixture=json.loads(fixture_path.read_text())
    inventory_path=ROOT/'docs/hyprland-feature-inventory.json'
    inventory=json.loads(inventory_path.read_text())
    if fixture['sources_sha256']!=digest(ROOT/'data/scalar-registration-sources.json') or fixture['audit_inventory_sha256']!=digest(inventory_path) or fixture['registration_metadata_matches_audit'] is not True:
        raise ValueError('Regenerate and verify source registration observations first')
    all_sources={'baseline':inventory['scalar_settings'],'current':inventory['current_source_comparison']['scalar_settings']}
    catalogue=json.loads((ROOT/'data/scalars.json').read_text())
    catalogue={(r['version'],r['id']):r for r in catalogue}
    contracts=json.loads((ROOT/'docs/hyprland-implementation-contracts.json').read_text())
    entries=[];findings=[];device_additions=[];scope_additions={}
    for snapshot in fixture['snapshots']:
        version=VERSIONS[snapshot['version']]
        sources={r['id']:r for r in all_sources[snapshot['version']]}
        parents={decoded(r['id_hex']):r for r in snapshot['values']}
        first_parent={}
        for parent_hex in snapshot['device_parent_names_hex']:
            parent=parents[decoded(parent_hex)]
            key=decoded(parent['lua_key_hex']).rsplit('.',1)[-1]
            first_parent.setdefault(key,parent)
        for value in snapshot['values']:
            identifier=decoded(value['id_hex']);source=sources[identifier]
            row=entry('global',version,identifier,value['kind'],value['lua'],source)
            row['lua_key']=decoded(value['lua_key_hex'])
            if row['lua_key']!=catalogue[(version,identifier)]['lua_key'] or value['kind']!=catalogue[(version,identifier)]['kind']:
                raise ValueError('Native registry key/type differs from catalogue')
            row['description']=decoded(value['description_hex'])
            row['deprecation']=None if value['deprecation_hex'] is None else decoded(value['deprecation_hex'])
            row['declared_bounds']={key:None if value.get(key) is None else number(value[key]) if value['kind']=='Float' else value[key] for key in ['minimum','maximum']}
            row['declared_names']=names(value.get('names'))
            choices=source_choices(source.get('metadata_cpp',''))
            row['declared_strings']=None if choices is None else [list(c) for c in choices]
            entries.append(row)
        for value in snapshot['device_values']:
            key=decoded(value['field_hex']);parent=first_parent.get(key)
            if key!='keybinds' and parent is None:raise ValueError('Device value lacks first declared parent')
            kind=parent['kind'] if parent else 'Bool'
            row=entry('device',version,key,kind,value['lua'],sources[decoded(parent['id_hex'])] if parent else None)
            row['parent_id']=decoded(parent['id_hex']) if parent else None
            entries.append(row)
        # Original scope remains fixed when product controls are later added.
        audited={r['source_versions'][snapshot['version']]['name'] for r in contracts['features'] if r['engine']=='DEVICE' and r['kind']=='structured_field' and snapshot['version'] in r['source_versions']}
        native={decoded(r['field_hex']) for r in snapshot['device_values']}
        missing=sorted(native-audited)
        if missing!=['eraser_button_mode','eraser_button_override','pressure_range_max','pressure_range_min'] or audited-native!={'tags'}:
            raise ValueError('Reconcile changed device audit scope explicitly')
        findings.append({'version':version,'kind':'device_fields_missing_from_original_register','fields':missing,'state':'required_unfinished','note':'Native value constructor has 46 fields; tags is handled separately, making 47 public fields before the name selector. Original device register has 43.'})
        for key in missing:
            parent=first_parent[key];identifier=decoded(parent['id_hex']);kind=parent['kind']
            if kind not in ['Int','Float']:raise ValueError('Review new device addition representation')
            def bound(name):
                value=parent.get(name)
                return None if value is None else number(value) if kind=='Float' else value
            default=parent['native_default']
            expression=repr(number(default)) if kind=='Float' else str(default)
            device_additions.append({'id':'device:'+key,'version':version,'kind':kind,'lua_key':key,
                'default_expression':expression,'bounds':{'min':bound('minimum'),'max':bound('maximum')},
                'source':sources[identifier]['source']})
            scope=scope_additions.setdefault('native-addition:DEVICE:'+key,{'id':'native-addition:DEVICE:'+key,'engine':'DEVICE','field':key,'versions':[],
                'requirement':'Complete per-device control, source codec, persistence/history, version-specific semantics, discovery/preview/apply/recovery and original release/security gates','complete':False})
            scope['versions'].append(version)
        for value in snapshot['runtime_registry_additions']:
            key=decoded(value['lua_key_hex'])
            row=entry('bootstrap',version,key,value['lua']['kind'],value['lua'])
            entries.append(row)
            findings.append({'version':version,'kind':'runtime_registration_missing_from_scalar_catalogue','lua_key':key,'state':'required_unfinished','note':'Separate generated-configuration marker registration. Its control/source-ownership handling and supplementary contract remain required.'})
            scope=scope_additions.setdefault('native-addition:SOURCE:'+key,{'id':'native-addition:SOURCE:'+key,'engine':'SOURCE','field':key,'versions':[],
                'requirement':'Complete generated-configuration marker control, source-ownership handling and original release/security gates','complete':False})
            scope['versions'].append(version)
    keys={(r['role'],r['version'],r['id']) for r in entries}
    if len(keys)!=len(entries) or len(entries)!=849:raise ValueError('Unexpected registration semantics scope')
    result={'format':'hye-scalar-semantics-1','registration_sha256':digest(fixture_path),
            'meaning':'Frozen registration models, not target runtime state or edit authority. Declared generic metadata differs from Lua-consumed constraints by version. All nine Lua constructor families have parser descriptors; complex defaults/derived caches remain unmodeled. Device rows exclude separately handled tags/name. Bootstrap entries supplement the original catalogue.',
            'entries':entries}
    outputs=[(OUTPUT,result),(FINDINGS,{'format':'hye-registration-findings-1','registration_sha256':digest(fixture_path),'original_contracts_preserved':1453,'findings':findings,'complete':False}),
             (DEVICE_ADDITIONS,device_additions),(SCOPE_ADDITIONS,{'format':'hye-source-additions-1','requirements':list(scope_additions.values())})]
    for path,value in outputs:
        encoded=json.dumps(value,indent=2)+'\n'
        if path==OUTPUT:
            header={k:v for k,v in value.items() if k!='entries'}
            encoded=json.dumps(header,separators=(',',':'))[:-1]+',\n"entries":[\n'+',\n'.join('  '+json.dumps(row,separators=(',',':')) for row in entries)+'\n]}\n'
        if path==OUTPUT and len(encoded.encode())>512*1024:raise ValueError('Runtime metadata exceeds 512 KiB')
        if args.check:
            if not path.exists() or path.read_text()!=encoded:raise SystemExit(f'{path.name} differs from verified registrations')
        else:path.write_text(encoded)
    print(f'{len(entries)} scoped records: 755 standard, 92 device, 2 bootstrap. Original omissions remain required.')


if __name__=='__main__':main()
