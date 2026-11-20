#!/usr/bin/env python3
"""Compare workspace references under explicit Lua/C++ fixture profiles.

Original callbacks run only trusted fixture Lua with the same explicit inert
dependencies as the argument/object probes. Actual environment metadata is
written separately from semantic fixtures. No selected compositor is observed.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import workspace_argument_probe as arguments
import workspace_object_probe as objects
from config_table_probe import SNAPSHOT, canonical
from monitor_source_probe import fetch, limits
from scalar_registration_probe import block

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/workspace-runtime-fixtures.json'
PROFILE = r'''
#include <lua.hpp>
#include <iostream>
#include <string>
#include <version>
#ifndef __cpp_lib_to_string
#define __cpp_lib_to_string 0
#endif
#ifndef __GLIBCXX__
#define __GLIBCXX__ 0
#endif
std::string hex(const char*s){const char*digits="0123456789abcdef";std::string out;for(;*s;++s){auto c=static_cast<unsigned char>(*s);out+=digits[c>>4];out+=digits[c&15];}return out;}
int main(){
 if(lua_version(nullptr)!=LUA_VERSION_NUM)return 3;
 std::cout<<R"({"cplusplus":)"<<__cplusplus<<R"(,"header_release":)"<<LUA_VERSION_RELEASE_NUM<<R"(,"runtime_abi":)"<<lua_version(nullptr)
 <<R"(,"libstdcpp_date":)"<<__GLIBCXX__<<R"(,"to_string_feature":)"<<__cpp_lib_to_string<<R"(,"example":")"<<std::to_string(1.25)
 <<R"(","negative_zero":")"<<std::to_string(-0.0)<<R"(","compiler_hex":")"<<hex(__VERSION__)<<R"(","identity_hex":")"<<hex(lua_ident)<<"\"}\n";
}
'''
EXTRA = [
    ('option_negative_zero', "{value=-0.0}"),
    ('option_integer_minimum', "{value=-9223372036854775807-1}"),
    ('option_small_float', "{value=1e-20}"),
    ('option_decimal_float', "{value=0.1}"),
    ('option_integer_boundary_float', "{value=9223372036854775808.0}"),
    ('option_largest_finite_float', "{value=1.7976931348623157e308}"),
]


def snapshot():
    code = SNAPSHOT.split('int observeConfig')[0]
    old = 'std::format(R"({{"type":"float","value":{}}})",lua_tonumber(L,index))'
    new = 'std::format(R"({{"type":"float","value":{{"bits":"{:016x}"}}}})",std::bit_cast<uint64_t>(lua_tonumber(L,index)))'
    if code.count(old) != 1:
        raise RuntimeError('Snapshot float encoding requires review')
    return code.replace(old, new)


def compile_fixture(root, name, code, flags, standard, compiler):
    path = root / (name + '.cpp')
    binary = root / name
    path.write_text('#include <bit>\n#include <cstdint>\n' + code)
    result = subprocess.run([compiler, '-std=' + standard, '-O0', '-Wall', '-Wextra', str(path), '-o', str(binary), *flags], capture_output=True, text=True, timeout=90)
    if result.returncode:
        raise RuntimeError(f'Fixture compilation refused for {name}: {result.stderr}')
    return binary


def sources():
    contracts = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    found = []
    for kind, feature in [('arguments', 'api:hl.workspace_rule'), ('objects', 'object-method:HL.WorkspaceRule:is_enabled')]:
        api = next(f for f in contracts['features'] if f['id'] == feature)
        for version, value in api['handler_versions'].items():
            found.append({'kind': kind, 'version': version,
                          'url': value['source'].split('#')[0].replace('github.com/', 'raw.githubusercontent.com/').replace('/blob/', '/'),
                          'sha256': value['file_sha256']})
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        bodies = list(pool.map(fetch, found))
    return list(zip(found, bodies))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--observation', type=Path)
    parser.add_argument('--cpp26-standard', choices=['c++26', 'c++2c'], default='c++26')
    parser.add_argument('--compiler', default='c++')
    args = parser.parse_args()
    tool_paths=['scripts/workspace_runtime_probe.py','scripts/workspace_argument_probe.py','scripts/workspace_object_probe.py','scripts/config_table_probe.py','scripts/monitor_source_probe.py','scripts/scalar_registration_probe.py','docs/hyprland-implementation-contracts.json']
    tool_sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in tool_paths}
    originals = sources()
    data, observed = {}, []
    with tempfile.TemporaryDirectory(prefix='hye-workspace-runtime-') as directory:
        root = Path(directory)
        for family in ['5.4', '5.5']:
            flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua' + family], text=True))
            for label, standard in [(23, 'c++23'), (26, args.cpp26_standard)]:
                name = f'lua{family.replace(".", "")}-cpp{label}'
                profiler = compile_fixture(root, name + '-profile', PROFILE, flags, standard,args.compiler)
                profile = json.loads(subprocess.check_output([str(profiler)], text=True, timeout=4, env={'LC_ALL': 'C'}))
                if label == 26 and profile['cplusplus'] <= 202302:
                    raise RuntimeError('Requested C++26 mode was not observed')
                if label == 23 and not 202100 <= profile['cplusplus'] <= 202302:
                    raise RuntimeError('Requested C++23 mode was not observed')
                release = profile['header_release']
                expected = f'Lua {release // 10000}.{release // 100 % 100}.{release % 100} '
                if expected not in bytes.fromhex(profile['identity_hex']).decode():
                    raise RuntimeError('Fixture header and runtime library identities disagree')
                text = profile['example']
                style = {'1.250000': 'fixed6', '1.25': 'general'}.get(text)
                if style is None:
                    raise RuntimeError('Unreviewed std::to_string behavior')
                if style == 'general' and profile['to_string_feature'] < 202306:
                    raise RuntimeError('General std::to_string lacks its feature declaration')
                expected_zero = '-0.000000' if style == 'fixed6' else '-0'
                if profile['negative_zero'] != expected_zero:
                    raise RuntimeError('Unreviewed signed-zero text conversion')
                profile.update({'lua_family': family, 'requested_cpp_standard': label, 'number_text_profile': style})
                observations = []
                for row, body in originals:
                    source = body.decode()
                    native = name + '-' + row['version'] + '-' + row['kind']
                    if row['kind'] == 'arguments':
                        if 'Internal::setMgrFn(L, mgr, "workspace_rule", hlWorkspaceRule);' not in source:
                            raise RuntimeError('Workspace registration requires review')
                        fields = arguments.declarations(source)
                        descriptors = 'const Desc WORKSPACE_RULE_FIELDS[]={' + ','.join('{"' + f['key'] + '",[]()->ILuaConfigValue*{return new ILuaConfigValue;},fixtureApply}' for f in fields) + '};\n'
                        code = arguments.HEADER + snapshot() + descriptors + block(source, 'static int hlWorkspaceRule(lua_State* L)') + arguments.MAIN
                        cases = arguments.cases(fields) + [(n, "return select('#',hl.workspace_rule{workspace='1',layout_opts=" + value + '})', 1, '') for n, value in EXTRA]
                    else:
                        methods = '\n'.join(block(source, signature) for signature in [
                            'static int workspaceRuleEq(lua_State* L)', 'static int workspaceRuleToString(lua_State* L)',
                            'static int workspaceRuleSetEnabled(lua_State* L)', 'static int workspaceRuleIsEnabled(lua_State* L)',
                            'static int workspaceRuleIndex(lua_State* L)', 'void Objects::CLuaWorkspaceRule::setup(lua_State* L)',
                            'void Objects::CLuaWorkspaceRule::push(lua_State* L, const SP<Config::CWorkspaceRule>& rule)'])
                        code = objects.HEADER + '\nstatic constexpr const char* MT="HL.WorkspaceRule";\n' + snapshot() + methods + objects.MAIN
                        cases = objects.CASES
                    binary = compile_fixture(root, native, code, flags, standard,args.compiler)
                    for case in cases:
                        case_name, lua = case[:2]
                        command = [str(binary)] + ([case[3]] if row['kind'] == 'arguments' else [])
                        result = subprocess.run(command, input=lua, text=True, capture_output=True, check=True, timeout=4, preexec_fn=limits, env={'LC_ALL': 'C'})
                        value = json.loads(result.stdout)
                        if row['kind'] == 'arguments':
                            value = arguments.normalized(value)
                            arguments.check_observation(case_name, value, case[2], text.encode())
                            if case_name.startswith('option_'):
                                options=value['added'][0]['layout_options_hex']
                                assert len(options)==1 and options[0][0]==b'value'.hex(),case_name
                                if case_name=='option_integer_minimum':
                                    assert options[0][1]==b'-9223372036854775808'.hex(),case_name
                                if case_name=='option_negative_zero':
                                    assert options[0][1]==expected_zero.encode().hex(),case_name
                                    table=next(v for k,v in value['arguments'][0]['value'] if k=={'type':'bytes','value':list(b'layout_opts')})
                                    assert table['value'][0][1]=={'type':'float','value':{'bits':'8000000000000000'}},case_name
                        else:
                            value['returns'] = [canonical(v) for v in value['returns']]
                            assert value['returns'] == case[2], case_name
                            assert value['setter_values'] == case[3], case_name
                            assert value['refresh_requests'] == case[4], case_name
                        observations.append({'version': row['version'], 'kind': row['kind'], 'case': case_name, 'lua': lua, 'observation': value})
                    profile.setdefault('fixture_binaries', {})[native] = hashlib.sha256(binary.read_bytes()).hexdigest()
                key = f'lua{family}-{style}'
                if key in data and data[key] != observations:
                    raise RuntimeError('Same semantic profile changed across C++ modes')
                data[key] = observations
                profile['cases_executed']=len(observations)
                observed.append(profile)
    if tool_sources!={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in tool_paths}:
        raise SystemExit('Reference tools changed during observation')
    content = {'format': 'hye-workspace-runtime-fixtures-1',
               'scope': 'Original workspace callbacks/userdata bodies with explicit inert dependencies under observed Lua/C++ fixture profiles. Floating arguments retain bits. Semantic groups distinguish fixed-six and general std::to_string; selected compositor/runtime, primary field acceptance, real manager/reload and generic registration policy remain unverified.',
               'sources': [row for row, _ in originals], 'profiles': data}
    rendered = json.dumps(content, indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if not OUTPUT.exists():
            raise SystemExit('Workspace runtime semantic reference drift')
        expected=json.loads(OUTPUT.read_text())
        if expected.get('format')!=content['format'] or expected.get('scope')!=content['scope'] or expected.get('sources')!=content['sources']:
            raise SystemExit('Workspace runtime reference context drift')
        for key,value in data.items():
            if expected['profiles'].get(key)!=value:
                raise SystemExit(f'Workspace runtime semantic reference drift: {key}')
    else:
        if set(data)!={f'lua{family}-{style}' for family in ['5.4','5.5'] for style in ['fixed6','general']}:
            raise SystemExit('Regeneration requires both text styles for both Lua families; existing reference coverage must not be discarded')
        OUTPUT.write_text(rendered)
    if args.observation:
        args.observation.write_text(json.dumps({'format': 'hye-workspace-runtime-observation-1', 'profiles': observed,'verified_semantic_profiles':sorted(data),'fixtures_sha256':hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),'tool_sources':tool_sources,'sources':[row for row,_ in originals], 'selected_compositor_observed': False}, indent=2) + '\n')
    print(f'Executed {sum(p["cases_executed"]for p in observed)} workspace runtime cases across {len(observed)} compiler/library contexts, comparing {len(data)} semantic groups; originals/builds removed')


if __name__ == '__main__':
    main()
