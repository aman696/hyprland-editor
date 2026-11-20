#!/usr/bin/env python3
"""Observe exact workspace-rule userdata methods with trusted private fixtures.

Weak references, userdata allocation and method bodies come from the pinned
object source. Rule state, refresh and metatable registration use inert adapters;
this does not certify real manager/reload lifetimes or compositor behavior.
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
#include <format>
#include <iostream>
#include <iterator>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <string_view>
#include <vector>
template<class T>using SP=std::shared_ptr<T>;
template<class T>using WP=std::weak_ptr<T>;
template<class T,class U>T sc(U value){return static_cast<T>(value);}
std::vector<bool>setterValues;
namespace Config {
 struct CWorkspaceRule {
  bool enabled=true;
  void setEnabled(bool v){setterValues.push_back(v);enabled=v;}
  bool isEnabled()const{return enabled;}
 };
 namespace Supplementary {
  constexpr int REFRESH_MONITOR_STATES=1,REFRESH_WINDOW_STATES=2;
  struct Refresher{int count=0;void scheduleRefresh(int){++count;}}refresh;
  Refresher*refresher(){return &refresh;}
 }
 namespace Lua::Objects {
  struct CLuaWorkspaceRule {
   static void setup(lua_State*);
   static void push(lua_State*,const SP<Config::CWorkspaceRule>&);
  };
 }
}
using namespace Config::Lua;
template<class T>int gcRef(lua_State*L){sc<T*>(lua_touserdata(L,1))->~T();return 0;}
// Registration is an explicit adapter, not a certificate of upstream helper policy.
void registerMetatable(lua_State*L,const char*name,lua_CFunction index,lua_CFunction gc,lua_CFunction eq,lua_CFunction text){
 luaL_newmetatable(L,name);
 for(const auto&[key,fn]:std::vector<std::pair<const char*,lua_CFunction>>{{"__index",index},{"__gc",gc},{"__eq",eq},{"__tostring",text}}){lua_pushcfunction(L,fn);lua_setfield(L,-2,key);}
 lua_pop(L,1);
}
'''
MAIN = r'''
std::map<std::string,SP<Config::CWorkspaceRule>>live;
int fixtureRef(lua_State*L){
 const std::string key=luaL_checkstring(L,1);const auto p=live.find(key);
 if(p==live.end())return luaL_error(L,"No live fixture rule");
 Objects::CLuaWorkspaceRule::push(L,p->second);return 1;
}
int fixtureDrop(lua_State*L){live.erase(luaL_checkstring(L,1));return 0;}
int main(){
 live["a"]=std::make_shared<Config::CWorkspaceRule>();live["b"]=std::make_shared<Config::CWorkspaceRule>();
 auto*L=luaL_newstate();if(!L)return 2;luaL_openlibs(L);Objects::CLuaWorkspaceRule::setup(L);
 lua_pushcfunction(L,fixtureRef);lua_setglobal(L,"ref");lua_pushcfunction(L,fixtureDrop);lua_setglobal(L,"drop");
 const std::string code((std::istreambuf_iterator<char>(std::cin)),{});
 if(luaL_loadbufferx(L,code.data(),code.size(),"@trusted-workspace-object-fixture","t")||lua_pcall(L,0,LUA_MULTRET,0))return 3;
 std::cout<<R"({"returns":[)";
 for(int i=1;i<=lua_gettop(L);++i){if(i>1)std::cout<<',';std::set<const void*>seen;int visits=0;std::cout<<snapshot(L,i,seen,0,visits);}
 std::cout<<R"(],"setter_values":[)";for(size_t i=0;i<setterValues.size();++i){if(i)std::cout<<',';std::cout<<(setterValues[i]?"true":"false");}
 std::cout<<R"(],"refresh_requests":)"<<Config::Supplementary::refresh.count<<R"(,"live_enabled":{)";
 bool first=true;for(const auto&[key,rule]:live){if(!first)std::cout<<',';first=false;std::cout<<'"'<<key<<"\":"<<(rule->enabled?"true":"false");}
 std::cout<<"}}\n";lua_close(L);
}
'''


def b(value):
    return {'type': 'bool', 'value': value}


def text(value):
    return {'type': 'bytes', 'value': list(value.encode())}


def integer(value):
    return {'type': 'int', 'value': value}


CASES = [
    ('initial', "local a=ref('a');return type(a),a:is_enabled()", [text('userdata'), b(True)], [], 0),
    ('disable', "local a=ref('a');return select('#',a:set_enabled(false)),a:is_enabled()", [integer(0), b(False)], [False], 1),
    ('enable_already_enabled', "local a=ref('a');return select('#',a:set_enabled(true)),a:is_enabled()", [integer(0), b(True)], [True], 1),
    ('get_extra_argument', "return ref('a'):is_enabled('ignored')", [b(True)], [], 0),
    ('set_extra_argument', "local a=ref('a');a:set_enabled(false,'ignored');return a:is_enabled()", [b(False)], [False], 1),
    ('wrong_set_type', "local a=ref('a');local ok,e=pcall(a.set_enabled,a,0);return ok,type(e),a:is_enabled()", [b(False), text('string'), b(True)], [], 0),
    ('missing_set_argument', "local a=ref('a');local ok,e=pcall(a.set_enabled,a);return ok,type(e),a:is_enabled()", [b(False), text('string'), b(True)], [], 0),
    ('wrong_self', "local a=ref('a');local ok,e=pcall(a.is_enabled,{});return ok,type(e)", [b(False), text('string')], [], 0),
    ('dot_call_missing_self', "local a=ref('a');local ok,e=pcall(a.set_enabled,false);return ok,type(e)", [b(False), text('string')], [], 0),
    ('unknown_member', "return type(ref('a').outside)", [text('nil')], [], 0),
    ('numeric_member', "return type(ref('a')[3])", [text('nil')], [], 0),
    ('bad_member_type', "local a=ref('a');local ok,e=pcall(function()return a[{}]end);return ok,type(e)", [b(False), text('string')], [], 0),
    ('nul_method_alias', "local a=ref('a');a['set_enabled\\0tail'](a,false);return a['is_enabled\\0tail'](a)", [b(False)], [False], 1),
    ('shared_rule_wrappers', "local a,b=ref('a'),ref('a');a:set_enabled(false);return rawequal(a,b),a==b,b:is_enabled()", [b(False), b(True), b(False)], [False], 1),
    ('different_rules', "return ref('a')==ref('b')", [b(False)], [], 0),
    ('unbound_method', "local a,b=ref('a'),ref('b');a.set_enabled(b,false);return a:is_enabled(),b:is_enabled()", [b(True), b(False)], [False], 1),
    ('live_tostring', "return string.match(tostring(ref('a')),'^HL%.WorkspaceRule%(')~=nil", [b(True)], [], 0),
    ('expired_get_and_set', "local a=ref('a');drop('a');return a:is_enabled(),select('#',a:set_enabled(false)),tostring(a)", [{'type': 'nil'}, integer(0), text('HL.WorkspaceRule(expired)')], [], 0),
    ('expired_wrong_set_type', "local a=ref('a');drop('a');local ok,e=pcall(a.set_enabled,a,0);return ok,type(e)", [b(False), text('string')], [], 0),
    ('both_expired_equal', "local a,b=ref('a'),ref('b');drop('a');drop('b');return a==b", [b(True)], [], 0),
    ('one_expired_different', "local a,b=ref('a'),ref('b');drop('a');return a==b", [b(False)], [], 0),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    contract = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    api = next(f for f in contract['features'] if f['id'] == 'object-method:HL.WorkspaceRule:is_enabled')
    setter = next(f for f in contract['features'] if f['id'] == 'object-method:HL.WorkspaceRule:set_enabled')
    rows = [{'version': version, 'url': value['source'].split('#')[0].replace('github.com/', 'raw.githubusercontent.com/').replace('/blob/', '/'),
             'sha256': value['file_sha256']} for version, value in api['handler_versions'].items()]
    for row in rows:
        assert setter['handler_versions'][row['version']]['file_sha256'] == row['sha256']
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        originals = list(pool.map(fetch, rows))
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
    observations, schemas = [], []
    with tempfile.TemporaryDirectory(prefix='hye-workspace-objects-') as directory:
        root = Path(directory)
        for row, body in zip(rows, originals):
            source = body.decode()
            metatable = re.search(r'static constexpr const char\* MT = "([^"]+)";', source)
            if not metatable or metatable[1] != 'HL.WorkspaceRule':
                raise RuntimeError('Unreviewed workspace object metatable')
            index = block(source, 'static int workspaceRuleIndex(lua_State* L)')
            routes = dict(re.findall(r'if \(key == "([^"]+)"\)\s*lua_pushcfunction\(L, ([A-Za-z0-9_]+)\);', index))
            expected = {'is_enabled': api['handler_versions'][row['version']]['handler_expression'],
                        'set_enabled': setter['handler_versions'][row['version']]['handler_expression']}
            if routes != expected:
                raise RuntimeError('Workspace object member routing requires review')
            schemas.append({'version': row['version'], 'source': row, 'metatable': metatable[1], 'methods': routes})
            methods = '\n'.join(block(source, signature) for signature in [
                'static int workspaceRuleEq(lua_State* L)', 'static int workspaceRuleToString(lua_State* L)',
                'static int workspaceRuleSetEnabled(lua_State* L)', 'static int workspaceRuleIsEnabled(lua_State* L)',
                'static int workspaceRuleIndex(lua_State* L)', 'void Objects::CLuaWorkspaceRule::setup(lua_State* L)',
                'void Objects::CLuaWorkspaceRule::push(lua_State* L, const SP<Config::CWorkspaceRule>& rule)'])
            cpp = root / (row['version'] + '.cpp')
            cpp.write_text(HEADER + '\n' + metatable[0] + '\n' + SNAPSHOT.split('int observeConfig')[0] + methods + MAIN)
            binary = root / row['version']
            compiled = subprocess.run(['c++', '-std=c++23', '-O0', '-Wall', '-Wextra', '-o', str(binary), str(cpp), *flags], capture_output=True, text=True, timeout=90)
            if compiled.returncode:
                raise RuntimeError(compiled.stderr)
            for name, lua, returns, setters, refreshes in CASES:
                result = subprocess.run([str(binary)], input=lua, text=True, capture_output=True, check=True, timeout=4, preexec_fn=limits, env={'LC_ALL': 'C'})
                value = json.loads(result.stdout)
                value['returns'] = [canonical(v) for v in value['returns']]
                assert value['returns'] == returns, (name, value['returns'])
                assert value['setter_values'] == setters, name
                assert value['refresh_requests'] == refreshes, name
                observations.append({'version': row['version'], 'case': name, 'lua': lua, 'observation': value})
    content = {'format': 'hye-workspace-object-fixtures-1', 'scope': 'Exact userdata allocation, weak-reference method bodies, equality, member lookup, required receiver/type checks and callback arity, using inert rule state, refresh and metatable registration adapters. Trusted ref/drop fixture functions model external object lifetime; no actual manager lifetime/reload, helper-protection policy, renderer, hardware or source-ownership certificate.', 'sources': rows, 'cases': observations}
    outputs = {ROOT / 'data/workspace-object-fixtures.json': content,
               ROOT / 'data/workspace-object-semantics.json': {'format': 'hye-workspace-object-semantics-1', 'scope': 'Original metatable and member routing; native rule lifetime, state/refresh effects and generic registration/protection remain unverified', 'snapshots': schemas}}
    for path, value in outputs.items():
        rendered = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        if args.check:
            if not path.exists() or path.read_text() != rendered:
                raise SystemExit(f'Workspace object reference drift: {path.relative_to(ROOT)}')
        else:
            path.write_text(rendered)
    print(f'Observed {len(observations)} pinned workspace object cases; originals/builds removed; manager/reload lifecycle remains unverified')


if __name__ == '__main__':
    main()
