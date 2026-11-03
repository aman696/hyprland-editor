#!/usr/bin/env python3
"""Observe pinned workspace-rule argument routing using trusted Lua fixtures.

Original callback code runs with inert parser, selector, manager, refresh and
returned-object adapters. Their behavior is not native conformance. Downloaded
source and compiled fixtures live only in a temporary directory.
"""
import argparse
import concurrent.futures
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from config_table_probe import SNAPSHOT, canonical
from monitor_source_probe import fetch, limits
from scalar_registration_probe import block

ROOT = Path(__file__).resolve().parents[1]
HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <format>
#include <iostream>
#include <iterator>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <tuple>
#include <vector>
template<class T>using UP=std::unique_ptr<T>;
template<class T,class U>T sc(U value){return static_cast<T>(value);}
std::string hex(std::string_view s){std::string out;for(unsigned char c:s)out+=std::format("{:02x}",c);return out;}
struct Result{int errorCode;std::string message;};
constexpr int PARSE_ERROR_OK=0;
constexpr long WORKSPACE_INVALID=-1;
struct CConfigManager{std::vector<std::string>errors;void addError(const std::string&s){errors.push_back(s);}};
namespace Config {
 struct CWorkspaceRule {
  std::string m_workspaceString,m_workspaceName;long m_workspaceId=-1;bool enabled=true;
  std::map<std::string,std::string>m_layoutopts;std::vector<std::string>fields;
  void setEnabled(bool v){enabled=v;}
 };
 namespace Supplementary{constexpr int REFRESH_WINDOW_STATES=2;}
}
struct Manager {
 std::vector<std::shared_ptr<Config::CWorkspaceRule>>added;
 auto replaceOrAdd(Config::CWorkspaceRule&&rule){auto p=std::make_shared<Config::CWorkspaceRule>(std::move(rule));added.push_back(p);return p;}
} manager;
namespace Config{Manager*workspaceRuleMgr(){return &manager;}}
// Selector parsing is a separate, deliberately inert adapter.
std::tuple<long,std::string,bool>getWorkspaceIDNameFromString(const std::string&s){return {-1,s,true};}
int objectPushes=0;
namespace Objects {
 struct CLuaWorkspaceRule {
  static void push(lua_State*L,const std::shared_ptr<Config::CWorkspaceRule>&rule){++objectPushes;lua_pushlightuserdata(L,rule.get());}
 };
}
std::string rejectParse,lastKey;
struct Delegation{std::string key,input;};
std::vector<Delegation>delegated;
std::vector<std::string>applied;
std::string snapshot(lua_State*,int,std::set<const void*>&,int,int&);
struct ILuaConfigValue {
 Result parse(lua_State*L){
  size_t n=0;const char*key=lua_tolstring(L,-2,&n);lastKey=key;
  std::set<const void*>seen;int visits=0;
  delegated.push_back({hex(std::string_view(key,n)),snapshot(L,-1,seen,0,visits)});
  return lastKey==rejectParse?Result{1,"fixture parser refusal"}:Result{0,""};
 }
};
void fixtureApply(ILuaConfigValue*,Config::CWorkspaceRule&r){applied.push_back(hex(lastKey));r.fields.push_back(hex(lastKey));}
struct Desc{const char*name;ILuaConfigValue*(*factory)();void(*apply)(ILuaConfigValue*,Config::CWorkspaceRule&);};
namespace Internal {
 std::string getSourceInfo(lua_State*){return "fixture";}
 template<size_t N>const Desc*findDescByName(const Desc(&descs)[N],std::string_view key){
  const auto found=std::ranges::find_if(descs,[key](const auto&d){return std::string_view(d.name)==key;});
  return found==std::end(descs)?nullptr:found;
 }
}
namespace Supplementary {
 constexpr int REFRESH_MONITOR_STATES=1;
 struct Refresher{int count=0;void scheduleRefresh(int){++count;}}refresh;
 Refresher*refresher(){return &refresh;}
}
'''
MAIN = r'''
int observeWorkspace(lua_State*L){
 for(int i=1;i<=lua_gettop(L);++i){std::set<const void*>seen;int visits=0;arguments.push_back(snapshot(L,i,seen,0,visits));}
 return hlWorkspaceRule(L);
}
void strings(const std::vector<std::string>&items){
 std::cout<<'[';bool first=true;for(const auto&v:items){if(!first)std::cout<<',';first=false;std::cout<<'"'<<v<<'"';}std::cout<<']';
}
int main(int argc,char**argv){
 if(argc!=2)return 2;rejectParse=argv[1];
 auto*L=luaL_newstate();if(!L)return 3;luaL_openlibs(L);CConfigManager self;
 lua_newtable(L);lua_pushlightuserdata(L,&self);lua_pushcclosure(L,observeWorkspace,1);lua_setfield(L,-2,"workspace_rule");lua_setglobal(L,"hl");
 lua_pushinteger(L,0);lua_setglobal(L,"__index_calls");lua_pushinteger(L,0);lua_setglobal(L,"__pairs_calls");
 const std::string code((std::istreambuf_iterator<char>(std::cin)),{});
 if(luaL_loadbufferx(L,code.data(),code.size(),"@trusted-workspace-fixture","t")||lua_pcall(L,0,LUA_MULTRET,0))return 4;
 std::cout<<R"({"arguments":[)";for(size_t i=0;i<arguments.size();++i){if(i)std::cout<<',';std::cout<<arguments[i];}
 std::cout<<R"(],"returns":[)";
 for(int i=1;i<=lua_gettop(L);++i){if(i>1)std::cout<<',';std::set<const void*>seen;int visits=0;std::cout<<snapshot(L,i,seen,0,visits);}
 std::cout<<R"(],"delegated":[)";
 for(size_t i=0;i<delegated.size();++i){if(i)std::cout<<',';std::cout<<R"({"key_hex":")"<<delegated[i].key<<R"(","input":)"<<delegated[i].input<<'}';}
 std::cout<<R"(],"applied_keys_hex":)";strings(applied);
 std::cout<<R"(,"added":[)";
 for(size_t i=0;i<manager.added.size();++i){if(i)std::cout<<',';const auto&r=*manager.added[i];
  std::cout<<R"({"workspace_hex":")"<<hex(r.m_workspaceString)<<R"(","enabled":)"<<(r.enabled?"true":"false")<<R"(,"fields_hex":)";strings(r.fields);
  std::cout<<R"(,"layout_options_hex":[)";bool first=true;for(const auto&[key,value]:r.m_layoutopts){if(!first)std::cout<<',';first=false;std::cout<<"[\""<<hex(key)<<"\",\""<<hex(value)<<"\"]";}std::cout<<"]}";
 }
 std::vector<std::string>errors;for(const auto&e:self.errors)errors.push_back(hex(e));std::cout<<R"(],"errors_hex":)";strings(errors);
 std::cout<<R"(,"refresh_requests":)"<<Supplementary::refresh.count<<R"(,"object_push_requests":)"<<objectPushes;
 lua_getglobal(L,"__index_calls");std::cout<<R"(,"index_calls":)"<<lua_tointeger(L,-1);lua_pop(L,1);
 lua_getglobal(L,"__pairs_calls");std::cout<<R"(,"pairs_calls":)"<<lua_tointeger(L,-1);lua_pop(L,1);
 std::cout<<"}\n";lua_close(L);
}
'''


def declarations(text):
    array = block(text, 'inline constexpr SWorkspaceRuleFieldDesc WORKSPACE_RULE_FIELDS[]')
    pattern = r'\{"([a-z_]+)",\s*\[\]\(\)\s*->\s*ILuaConfigValue\*\s*\{\s*return new CLuaConfig([A-Za-z0-9]+)\(([^()]*)\);\s*\}'
    fields = []
    for key, kind, args in re.findall(pattern, array):
        args = [arg.strip() for arg in args.split(',')]
        if kind not in {'Bool', 'Int', 'String', 'CssGap'} or not 1 <= len(args) <= 3:
            raise RuntimeError('Unreviewed workspace factory shape')
        bounds = [None, None]
        for index, arg in enumerate(args[1:]):
            if not re.fullmatch(r'-?\d+', arg):
                raise RuntimeError('Nonliteral workspace bound requires review')
            bounds[index] = int(arg)
        fields.append({'key': key, 'kind': kind, 'declared_default_cpp': args[0],
                       'minimum': bounds[0], 'maximum': bounds[1]})
    if len(fields) != 15 or len({f['key'] for f in fields}) != 15:
        raise RuntimeError('Workspace field extraction is incomplete')
    return fields


def cases(fields):
    # The expected result is callback arity, not a native object implementation.
    result = [(name, f"return select('#',hl.workspace_rule{args})", count, '') for name, args, count in [
        ('missing_argument', '()', 0), ('nil_argument', '(nil)', 0), ('number_argument', '(3)', 0),
        ('missing_workspace', '{}', 0), ('boolean_workspace', '{workspace=true}', 0),
        ('nul_required_key', "{['workspace\\0tail']='1'}", 0),
        ('empty_workspace', "{workspace=''}", 1), ('number_workspace', '{workspace=3}', 1),
        ('float_workspace', '{workspace=3.25}', 1), ('nul_workspace', "{workspace='name:a\\0tail'}", 1),
        ('disabled', "{workspace='1',enabled=false}", 1), ('numeric_enabled_ignored', "{workspace='1',enabled=0}", 1),
        ('unknown_and_nonstring_fields', "{workspace='1',outside=3,[1]=4,[false]=5}", 1),
        ('nul_known_key', "{workspace='1',['border_size\\0tail']=3}", 1),
        ('bad_options_container', "{workspace='1',layout_opts=3,border_size=4}", 1),
        ('typed_options', "{workspace='1',layout_opts={text='a',yes=true,no=false,integer=7,float=1.25}}", 1),
        ('options_nul_bytes', "{workspace='1',layout_opts={['key\\0tail']='value\\0tail'}}", 1),
        ('bad_option_values_continue', "{workspace='1',layout_opts={good='yes',bad={},fn=function()end}}", 1),
        ('bad_option_keys_continue', "{workspace='1',layout_opts={[1]='bad',ok=false}}", 1),
        ('extra_arguments', "({workspace='1'},{workspace='ignored'})", 1),
    ]]
    result += [
        ('workspace_index_lookup', "local t=setmetatable({border_size=3},{__index=function(_,k)__index_calls=__index_calls+1;if k=='workspace' then return '1' end end});return select('#',hl.workspace_rule(t))", 1, ''),
        ('enabled_index_lookup', "local t=setmetatable({workspace='1'},{__index=function(_,k)__index_calls=__index_calls+1;if k=='enabled' then return false end end});return select('#',hl.workspace_rule(t))", 1, ''),
        ('pairs_ignored', "local t=setmetatable({workspace='1',border_size=3},{__pairs=function()__pairs_calls=__pairs_calls+1;return next,{},nil end});return select('#',hl.workspace_rule(t))", 1, ''),
        ('fixture_parse_failure_continues', "return select('#',hl.workspace_rule{workspace='1',border_size=3,no_border=true})", 1, 'border_size'),
    ]
    values = {'Bool': 'true', 'Int': '3', 'String': "'sample'", 'CssGap': '{top=-1,left=2}'}
    result += [('field_' + f['key'], "return select('#',hl.workspace_rule{workspace='1'," + f['key'] + '=' + values[f['kind']] + '})', 1, '') for f in fields]
    return result


def normalized(value):
    value['arguments'] = [canonical(v) for v in value['arguments']]
    value['delegated'].sort(key=lambda row: (row['key_hex'], json.dumps(canonical(row['input']), sort_keys=True)))
    value['applied_keys_hex'].sort()
    value['errors_hex'].sort()
    for rule in value['added']:
        rule['fields_hex'].sort()
    return value


def check_observation(name, value, arity, number_text=b'1.250000'):
    assert value['returns'] == [{'type': 'int', 'value': arity}], name
    assert value['object_push_requests'] == arity, name
    assert value['refresh_requests'] == arity, name
    assert len(value['added']) == arity, name
    if not arity:
        assert value['errors_hex'] and not value['delegated'], name
        return
    rule = value['added'][0]
    assert rule['enabled'] == (name not in {'disabled', 'enabled_index_lookup'}), name
    if name.startswith('field_'):
        key = name.removeprefix('field_').encode().hex()
        assert value['applied_keys_hex'] == [key], name
        assert [r['key_hex'] for r in value['delegated']] == [key], name
        assert not value['errors_hex'], name
    elif name == 'fixture_parse_failure_continues':
        assert value['applied_keys_hex'] == [b'no_border'.hex()], name
        assert len(value['delegated']) == 2 and len(value['errors_hex']) == 1, name
    elif name == 'typed_options':
        assert {bytes.fromhex(k): bytes.fromhex(v) for k, v in rule['layout_options_hex']} == {
            b'text': b'a', b'yes': b'true', b'no': b'false', b'integer': b'7', b'float': number_text}, name
    elif name == 'options_nul_bytes':
        assert rule['layout_options_hex'] == [[b'key'.hex(), b'value'.hex()]], name
    elif name in {'number_workspace', 'float_workspace', 'nul_workspace'}:
        expected = {'number_workspace': b'3', 'float_workspace': b'3.25', 'nul_workspace': b'name:a'}
        assert rule['workspace_hex'] == expected[name].hex(), name
    elif name == 'pairs_ignored':
        assert value['pairs_calls'] == 0 and value['applied_keys_hex'] == [b'border_size'.hex()], name
    elif name in {'workspace_index_lookup', 'enabled_index_lookup'}:
        assert value['index_calls'] > 0, name


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    contract = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    api = next(f for f in contract['features'] if f['id'] == 'api:hl.workspace_rule')
    rows = [{'version': version, 'url': value['source'].split('#')[0].replace('github.com/', 'raw.githubusercontent.com/').replace('/blob/', '/'),
             'sha256': value['file_sha256']} for version, value in api['handler_versions'].items()]
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        originals = list(pool.map(fetch, rows))
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
    observations, schemas = [], []
    with tempfile.TemporaryDirectory(prefix='hye-workspace-arguments-') as directory:
        root = Path(directory)
        for row, body in zip(rows, originals):
            text = body.decode()
            if 'Internal::setMgrFn(L, mgr, "workspace_rule", hlWorkspaceRule);' not in text:
                raise RuntimeError('Workspace-rule registration requires review')
            fields = declarations(text)
            catalogue_version = '0.56.2' if row['version'] == 'baseline' else 'audited-main'
            audited = [f for f in json.loads((ROOT / 'data/workspaces.json').read_text()) if f['version'] == catalogue_version]
            assert {(f['lua_key'], f['kind']) for f in audited} == {(f['key'], f['kind']) for f in fields}
            schemas.append({'version': row['version'], 'source': row, 'fields': fields})
            descriptors = 'const Desc WORKSPACE_RULE_FIELDS[]={' + ','.join('{"' + f['key'] + '",[]()->ILuaConfigValue*{return new ILuaConfigValue;},fixtureApply}' for f in fields) + '};\n'
            source = root / (row['version'] + '.cpp')
            source.write_text(HEADER + SNAPSHOT.split('int observeConfig')[0] + descriptors + block(text, 'static int hlWorkspaceRule(lua_State* L)') + MAIN)
            binary = root / row['version']
            compiled = subprocess.run(['c++', '-std=c++23', '-O0', '-Wall', '-Wextra', '-o', str(binary), str(source), *flags], capture_output=True, text=True, timeout=90)
            if compiled.returncode:
                raise RuntimeError(compiled.stderr)
            for name, lua, arity, reject in cases(fields):
                result = subprocess.run([str(binary), reject], input=lua, text=True, capture_output=True, check=True, timeout=4, preexec_fn=limits, env={'LC_ALL': 'C'})
                observation = normalized(json.loads(result.stdout))
                check_observation(name, observation, arity)
                observations.append({'version': row['version'], 'case': name, 'lua': lua,
                                     'fixture_reject_parse': reject, 'observation': observation})
    outputs = {
        ROOT / 'data/workspace-argument-semantics.json': {'format': 'hye-workspace-argument-semantics-1', 'scope': 'Declared factories, not effective/default values or actual application', 'snapshots': schemas},
        ROOT / 'data/workspace-argument-fixtures.json': {'format': 'hye-workspace-argument-fixtures-1', 'scope': 'Original argument routing, required-field coercion, optional enabled lookup, layout-option conversion, continuation and callback arity, with inert field parsing/apply, selector, manager, refresh and returned-object adapters. Delegation multisets only; no native field acceptance, selector semantics, consolidation, object lifecycle, runtime or source-ownership certificate.', 'sources': rows, 'cases': observations},
    }
    for path, value in outputs.items():
        rendered = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        if args.check:
            if not path.exists() or path.read_text() != rendered:
                raise SystemExit(f'Workspace argument reference drift: {path.relative_to(ROOT)}')
        else:
            path.write_text(rendered)
    print(f'Observed {len(observations)} pinned workspace argument cases; originals/builds removed; native managers/objects remain unverified')


if __name__ == '__main__':
    main()
