#!/usr/bin/env python3
"""Observe pinned hl.monitor dispatch with inert field/service adapters.

Only reviewed fixture Lua runs. Value parsing, monitor parser/application,
manager replacement and physical refresh are deliberately NOT certified here.
Original handler files and native builds are deleted on every exit path.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import resource
import shlex
import subprocess
import sys
import tempfile
import urllib.request

sys.dont_write_bytecode = True
from scalar_registration_probe import block
from config_table_probe import SNAPSHOT, canonical

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/monitor-source-fixtures.json'
SEMANTICS = ROOT / 'data/monitor-source-semantics.json'
HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <cstdint>
#include <format>
#include <iostream>
#include <iterator>
#include <memory>
#include <set>
#include <string>
#include <vector>
template<class T>using UP=std::unique_ptr<T>;
template<class T,class U>T sc(U value){return static_cast<T>(value);}
std::string hex(std::string_view s){std::string out;for(unsigned char c:s)out+=std::format("{:02x}",c);return out;}
struct Result{int errorCode;std::string message;};
constexpr int PARSE_ERROR_OK=0;
struct Rule{std::string m_name;int inherited=0;std::vector<std::string> fields;};
struct Manager{
 std::vector<Rule> rules,added;
 const std::vector<Rule>&all()const{return rules;}
 void add(Rule&&rule){added.push_back(std::move(rule));}
} manager;
namespace Config{Manager*monitorRuleMgr(){return &manager;}}
struct CMonitorRuleParser{
 Rule value;explicit CMonitorRuleParser(const std::string&output){value.m_name=output;}
 Rule&rule(){return value;}
};
struct CConfigManager{std::vector<std::string>errors;void addError(const std::string&s){errors.push_back(s);}};
std::string rejectParse,rejectApply,lastKey;
struct Delegation{std::string key,input;};
std::vector<Delegation> delegated;
std::vector<std::string> applied;
std::string snapshot(lua_State*,int,std::set<const void*>&,int,int&);
struct ILuaConfigValue{
 Result parse(lua_State*L){
  size_t n=0;const char*key=lua_tolstring(L,-2,&n);lastKey=key;
  std::set<const void*>seen;int visits=0;
  delegated.push_back({hex(std::string_view(key,n)),snapshot(L,-1,seen,0,visits)});
  return lastKey==rejectParse?Result{1,"fixture parser refusal"}:Result{0,""};
 }
};
bool fixtureApply(ILuaConfigValue*,CMonitorRuleParser&p){
 applied.push_back(hex(lastKey));
 if(lastKey==rejectApply)return false;
 p.rule().fields.push_back(hex(lastKey));return true;
}
struct Desc{const char*name;ILuaConfigValue*(*factory)();bool(*apply)(ILuaConfigValue*,CMonitorRuleParser&);};
namespace Internal{
 std::string getSourceInfo(lua_State*){return "fixture";}
 template<size_t N>const Desc*findDescByName(const Desc(&descs)[N],const char*key){
  const auto found=std::ranges::find_if(descs,[key](const auto&d){return std::string_view(d.name)==key;});
  return found==std::end(descs)?nullptr:found;
 }
}
namespace Supplementary{
 constexpr int REFRESH_MONITOR_STATES=1;
 struct Refresher{int count=0;void scheduleRefresh(int){++count;}} refresh;
 Refresher*refresher(){return &refresh;}
}
'''
MAIN = r'''
int observeMonitor(lua_State*L){
 for(int i=1;i<=lua_gettop(L);++i){std::set<const void*>seen;int visits=0;arguments.push_back(snapshot(L,i,seen,0,visits));}
 return hlMonitor(L);
}
void strings(const std::vector<std::string>&items){
 std::cout<<'[';bool first=true;for(const auto&v:items){if(!first)std::cout<<',';first=false;std::cout<<'"'<<v<<'"';}std::cout<<']';
}
int main(int argc,char**argv){
 if(argc!=4)return 2;
 rejectParse=argv[1];rejectApply=argv[2];
 if(std::string(argv[3])=="seed")manager.rules.push_back(Rule{"SEED",77,{"73656564"}});
 auto*L=luaL_newstate();if(!L)return 3;luaL_openlibs(L);
 CConfigManager self;
 lua_newtable(L);lua_pushlightuserdata(L,&self);lua_pushcclosure(L,observeMonitor,1);lua_setfield(L,-2,"monitor");lua_setglobal(L,"hl");
 lua_pushinteger(L,0);lua_setglobal(L,"__index_calls");lua_pushinteger(L,0);lua_setglobal(L,"__pairs_calls");
 const std::string code((std::istreambuf_iterator<char>(std::cin)),{});
 if(luaL_loadbufferx(L,code.data(),code.size(),"@trusted-monitor-fixture","t")||lua_pcall(L,0,LUA_MULTRET,0))return 4;
 std::cout<<R"({"arguments":[)";for(size_t i=0;i<arguments.size();++i){if(i)std::cout<<',';std::cout<<arguments[i];}
 std::cout<<R"(],"returns":[)";
 for(int i=1;i<=lua_gettop(L);++i){if(i>1)std::cout<<',';std::set<const void*>seen;int visits=0;std::cout<<snapshot(L,i,seen,0,visits);}
 std::cout<<R"(],"delegated":[)";
 for(size_t i=0;i<delegated.size();++i){if(i)std::cout<<',';std::cout<<R"({"key_hex":")"<<delegated[i].key<<R"(","input":)"<<delegated[i].input<<'}';}
 std::cout<<R"(],"applied_keys_hex":)";strings(applied);
 std::cout<<R"(,"added":[)";
 for(size_t i=0;i<manager.added.size();++i){if(i)std::cout<<',';const auto&r=manager.added[i];std::cout<<R"({"output_hex":")"<<hex(r.m_name)<<R"(","inherited_seed":)"<<r.inherited<<R"(,"fields_hex":)";strings(r.fields);std::cout<<'}';}
 std::vector<std::string>errors;for(const auto&e:self.errors)errors.push_back(hex(e));
 std::cout<<R"(],"errors_hex":)";strings(errors);
 std::cout<<R"(,"refresh_requests":)"<<Supplementary::refresh.count;
 lua_getglobal(L,"__index_calls");std::cout<<R"(,"index_calls":)"<<lua_tointeger(L,-1);lua_pop(L,1);
 lua_getglobal(L,"__pairs_calls");std::cout<<R"(,"pairs_calls":)"<<lua_tointeger(L,-1);lua_pop(L,1);
 std::cout<<"}\n";lua_close(L);
}
'''


def declarations(text):
    array = block(text, 'inline constexpr SMonitorFieldDesc MONITOR_FIELDS[]')
    records = []
    pattern = r'\{"([a-z_]+)",\s*\[\]\(\)\s*->\s*ILuaConfigValue\*\s*\{\s*return new CLuaConfig([A-Za-z0-9]+)\(([^()]*)\);\s*\}'
    for key, kind, args in re.findall(pattern, array):
        args = [arg.strip() for arg in args.split(',')]
        if kind not in {'Bool', 'Int', 'Float', 'String', 'CssGap'} or not 1 <= len(args) <= 3:
            raise RuntimeError('Unreviewed monitor factory shape')
        bounds = [None, None]
        for index, arg in enumerate(args[1:]):
            if not re.fullmatch(r'-?\d+', arg):
                raise RuntimeError('Nonliteral monitor bound requires review')
            bounds[index] = int(arg)
        records.append({'key': key, 'kind': kind, 'declared_default_cpp': args[0],
                        'minimum': bounds[0], 'maximum': bounds[1]})
    if len(records) != 22 or len({row['key'] for row in records}) != 22:
        raise RuntimeError('Monitor field extraction is incomplete')
    return records


def fetch(row):
    with urllib.request.urlopen(row['url'], timeout=30) as response:
        body = response.read(512 * 1024 + 1)
    if hashlib.sha256(body).hexdigest() != row['sha256']:
        raise RuntimeError('Pinned monitor handler identity changed')
    return body


def limits():
    resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
    resource.setrlimit(resource.RLIMIT_STACK, (1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def cases(fields):
    result = [
        ('missing_argument', "return select('#',hl.monitor())"),
        ('nil_argument', "return select('#',hl.monitor(nil))"),
        ('number_argument', "return select('#',hl.monitor(3))"),
        ('missing_output', "return select('#',hl.monitor{})"),
        ('boolean_output', "return select('#',hl.monitor{output=true})"),
        ('empty_output', "return select('#',hl.monitor{output=''})"),
        ('number_output', "return select('#',hl.monitor{output=123})"),
        ('float_output', "return select('#',hl.monitor{output=123.25})"),
        ('nul_output', "return select('#',hl.monitor{output='DP-1\\0suffix'})"),
        ('nul_output_key_is_not_required_output', "return select('#',hl.monitor{['output\\0tail']='DP-1'})"),
        ('extra_arguments', "return select('#',hl.monitor({output='DP-1',mode='preferred'},{output='IGNORED'}))"),
        ('unknown_and_nonstring_fields', "return select('#',hl.monitor{output='DP-1',outside=1,[1]=2,[false]=3})"),
        ('nul_known_key', "return select('#',hl.monitor{output='DP-1',['mode\\0tail']='preferred'})"),
        ('nul_known_collision', "return select('#',hl.monitor{output='DP-1',mode='preferred',['mode\\0tail']='highrr'})"),
        ('reserved_aliases', "return select('#',hl.monitor{output='DP-1',reserved=-1,reserved_area=2})"),
        ('seeded_existing_rule', "return select('#',hl.monitor{output='SEED',disabled=true})"),
        ('unknown_field_does_not_abort', "return select('#',hl.monitor{output='DP-1',outside=1,transform=2})"),
        ('output_index_lookup', "local t=setmetatable({mode='preferred'},{__index=function(_,key)__index_calls=__index_calls+1;if key=='output' then return 'DP-1' end end});return select('#',hl.monitor(t))"),
        ('pairs_is_ignored', "local t=setmetatable({output='DP-1',mode='preferred'},{__pairs=function()__pairs_calls=__pairs_calls+1;return next,{},nil end});return select('#',hl.monitor(t))"),
    ]
    values = {'Bool': 'true', 'Int': '2', 'Float': '1.5', 'String': "'sample'", 'CssGap': '{top=-1,left=2}'}
    for row in fields:
        result.append(('field_' + row['key'], "return select('#',hl.monitor{output='DP-1'," + row['key'] + '=' + values[row['kind']] + '})'))
    return result


def normalized(value):
    value['arguments'] = [canonical(row) for row in value['arguments']]
    value['delegated'].sort(key=lambda row: (row['key_hex'], json.dumps(canonical(row['input']), sort_keys=True)))
    value['applied_keys_hex'].sort()
    value['errors_hex'].sort()
    for rule in value['added']:
        rule['fields_hex'].sort()
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    contract = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    api = next(row for row in contract['features'] if row['id'] == 'api:hl.monitor')
    rows = [{'version': version, 'url': value['source'].split('#')[0].replace('github.com/', 'raw.githubusercontent.com/').replace('/blob/', '/'),
             'sha256': value['file_sha256']} for version, value in api['handler_versions'].items()]
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        originals = list(pool.map(fetch, rows))
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
    data, schemas = [], []
    with tempfile.TemporaryDirectory(prefix='hye-monitor-reference-') as directory:
        root = Path(directory)
        for row, body in zip(rows, originals):
            text = body.decode(); fields = declarations(text)
            audited = [f for f in json.loads((ROOT / 'data/monitors.json').read_text()) if f['version'] == ('0.56.2' if row['version'] == 'baseline' else 'audited-main')]
            assert {(f['lua_key'], f['kind']) for f in audited} == {(f['key'], f['kind']) for f in fields}
            schemas.append({'version': row['version'], 'source': row, 'fields': fields})
            descriptors = 'const Desc MONITOR_FIELDS[]={' + ','.join('{"' + f['key'] + '",[]()->ILuaConfigValue*{return new ILuaConfigValue;},fixtureApply}' for f in fields) + '};\n'
            code = HEADER + SNAPSHOT.split('int observeConfig')[0] + descriptors + block(text, 'static int hlMonitor(lua_State* L)') + MAIN
            source = root / (row['version'] + '.cpp'); source.write_text(code)
            binary = root / row['version']
            subprocess.run(['c++', '-std=c++23', '-O0', '-Wall', '-Wextra', '-o', str(binary), str(source), *flags], check=True, capture_output=True, timeout=90)
            run_cases = [(name, lua, '', '') for name, lua in cases(fields)] + [
                ('fixture_parse_failure_continues', "return select('#',hl.monitor{output='DP-1',mode='preferred',transform=2})", 'mode', ''),
                ('fixture_apply_failure_continues', "return select('#',hl.monitor{output='DP-1',mode='preferred',transform=2})", '', 'mode')]
            for name, lua, reject_parse, reject_apply in run_cases:
                result = subprocess.run([str(binary), reject_parse, reject_apply, 'seed' if name == 'seeded_existing_rule' else 'empty'], input=lua, text=True, capture_output=True, check=True, timeout=4, preexec_fn=limits, env={'LC_ALL': 'C'})
                observation = normalized(json.loads(result.stdout))
                assert observation['returns'] == [{'type': 'int', 'value': 0}], name
                data.append({'version': row['version'], 'case': name, 'lua': lua, 'fixture_reject_parse': reject_parse,
                             'fixture_reject_apply': reject_apply, 'observation': observation})
    outputs = {
        OUTPUT: {'format': 'hye-monitor-source-fixtures-1', 'scope': 'Original hl.monitor routing/output coercion/continuation with inert field parser, apply, manager and refresh adapters. Delegation multisets only; no native field acceptance, parser/helper effects, real replacement/order, runtime/hardware or source ownership certificate.', 'sources': rows, 'cases': data},
        SEMANTICS: {'format': 'hye-monitor-source-semantics-1', 'scope': 'Declared factory arguments, not effective/default/application values', 'snapshots': schemas},
    }
    for path, content in outputs.items():
        rendered = json.dumps(content, indent=2, ensure_ascii=False) + '\n'
        if args.check:
            if not path.exists() or path.read_text() != rendered:
                raise SystemExit(f'Monitor reference drift: {path.relative_to(ROOT)}')
        else:
            path.write_text(rendered)
    print(f'Observed {len(data)} pinned monitor dispatch cases; originals/builds removed; native effects remain unverified')


if __name__ == '__main__':
    main()
