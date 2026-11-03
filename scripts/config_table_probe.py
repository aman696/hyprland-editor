#!/usr/bin/env python3
"""Observe exact pinned hl.config traversal; the inert value adapter is NOT a codec.

Only reviewed private fixtures execute Lua. Original source/build files are temporary.
The probe preserves delegation multisets, not a portable Lua table iteration order.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import resource
import shlex
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/config-table-fixtures.json'
CASES = [
    ('nested_scalar', 'hl.config{general={border_size=3}}'),
    ('flat_scalar', "hl.config{['general.border_size']=3}"),
    ('nil_argument', 'hl.config(nil)'),
    ('number_argument', 'hl.config(3)'),
    ('string_argument', "hl.config('table')"),
    ('no_argument', 'hl.config()'),
    ('extra_arguments', 'hl.config({general={border_size=3}},{general={border_size=9}})'),
    ('known_table_value', "hl.config{general={['col.active_border']={colors={'red'},angle=45}}}"),
    ('known_scalar_table_value', 'hl.config{general={border_size={nested=9}}}'),
    ('unknown_scalar', 'hl.config{outside=4}'),
    ('unknown_nested', 'hl.config{outside={a=1,b={c=2}}}'),
    ('empty_unknown_table', 'hl.config{outside={}}'),
    ('nonstring_keys', 'hl.config{[1]={general={border_size=7}},[true]=8,[function()end]=9}'),
    ('nul_key_alias', "hl.config{['general.border_size\\0suffix']=9}"),
    ('nul_category_alias', "hl.config{['general\\0suffix']={border_size=9}}"),
    ('nul_key_collision', "hl.config{['general.border_size\\0suffix']=9,['general.border_size']=3}"),
    ('dotted_nested_collision', "hl.config{general={border_size=3},['general.border_size']=9}"),
    ('empty_root_category', "hl.config{['']={general={border_size=3}}}"),
    ('empty_nested_category', "hl.config{general={['']={border_size=3}}}"),
    ('nul_leading_key', "hl.config{['\\0general.border_size']=3}"),
    ('raw_iteration_ignores_pairs', "local t=setmetatable({general={border_size=3}},{__pairs=function()__pairs_calls=__pairs_calls+1;return next,{},nil end});hl.config(t)"),
    ('nil_fields_not_stored', 'hl.config{general={border_size=nil}}'),
    ('string_value_bytes', "hl.config{general={border_size='3\\0suffix'}}"),
    ('float_value_delegation', 'hl.config{general={border_size=3.0}}'),
    ('failed_parse_continues', 'hl.config{general={border_size=3,gaps_out=7}}'),
    ('static_parse_no_refresh', 'hl.config{general={border_size=3,gaps_out=7}}'),
]
HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <bit>
#include <cstdint>
#include <format>
#include <functional>
#include <iostream>
#include <iterator>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>
template<class T,class U>T sc(U v){return static_cast<T>(v);}
std::string hex(std::string_view bytes){std::string out;for(unsigned char c:bytes)out+=std::format("{:02x}",c);return out;}
struct Delegation{std::string key,type,data;};
struct Result{int errorCode;std::string message;};
constexpr int PARSE_ERROR_OK=0;
struct CConfigManager;
struct Value{
 CConfigManager* owner;std::string key;
 Result parse(lua_State* L);
 int refreshBits(){return 1;} // an inert refresh token, not a real value's policy
};
struct CConfigManager{
 std::map<std::string,std::unique_ptr<Value>> m_configValues;
 std::vector<Delegation> calls;std::vector<std::string> errors;
 bool dynamic=true,failBorder=false;
 void addError(const std::string&s){errors.push_back(s);}
 bool isDynamicParse(){return dynamic;}
 CConfigManager(){for(const std::string key:{"general.border_size","general.gaps_out","general.col.active_border"})m_configValues[key]=std::make_unique<Value>(Value{this,key});}
};
Result Value::parse(lua_State*L){
 std::string type=lua_typename(L,lua_type(L,-1)),data;
 if(lua_type(L,-1)==LUA_TNUMBER){
  if(lua_isinteger(L,-1)){type="integer";data=std::to_string(lua_tointeger(L,-1));}
  else{type="float";data=std::format("{:016x}",std::bit_cast<uint64_t>(lua_tonumber(L,-1)));}
 }else if(lua_type(L,-1)==LUA_TSTRING){size_t n;const char*s=lua_tolstring(L,-1,&n);data=hex(std::string_view(s,n));}
 else if(lua_type(L,-1)==LUA_TBOOLEAN)data=lua_toboolean(L,-1)?"true":"false";
 owner->calls.push_back({key,type,data});
 return {owner->failBorder&&key=="general.border_size"?1:0,"fixture rejection"};
}
namespace Internal{std::string getSourceInfo(lua_State*){return "fixture";}}
namespace Supplementary{struct Refresher{int requests=0;void scheduleRefresh(int){requests++;}}r;Refresher*refresher(){return &r;}}
'''
SNAPSHOT = r'''
std::vector<std::string> arguments;
std::string snapshot(lua_State*L,int index,std::set<const void*>&seen,int depth,int&visits){
 if(depth>32||++visits>4096)throw std::runtime_error("fixture snapshot budget");
 index=lua_absindex(L,index);
 switch(lua_type(L,index)){
 case LUA_TNIL:return R"({"type":"nil"})";
 case LUA_TBOOLEAN:return std::format(R"({{"type":"bool","value":{}}})",lua_toboolean(L,index)?"true":"false");
 case LUA_TNUMBER:return lua_isinteger(L,index)?std::format(R"({{"type":"int","value":{}}})",lua_tointeger(L,index)):std::format(R"({{"type":"float","value":{}}})",lua_tonumber(L,index));
 case LUA_TSTRING:{size_t n;const char*s=lua_tolstring(L,index,&n);std::string bytes;for(size_t i=0;i<n;i++){if(i)bytes+=",";bytes+=std::to_string(static_cast<unsigned char>(s[i]));}return R"({"type":"bytes","value":[)"+bytes+"]}";}
 case LUA_TTABLE:{
  const void*id=lua_topointer(L,index);if(!seen.insert(id).second)return R"({"type":"cycle","value":0})";
  std::string rows;lua_pushnil(L);while(lua_next(L,index)!=0){if(!rows.empty())rows+=",";rows+="["+snapshot(L,-2,seen,depth+1,visits)+","+snapshot(L,-1,seen,depth+1,visits)+"]";lua_pop(L,1);}seen.erase(id);return R"({"type":"table","value":[)"+rows+"]}";
 }
 case LUA_TFUNCTION:return R"({"type":"function","value":"fixture-function"})";
 default:throw std::runtime_error("unmodeled fixture input type");
 }
}
int observeConfig(lua_State*L){
 for(int i=1;i<=lua_gettop(L);i++){std::set<const void*>seen;int visits=0;arguments.push_back(snapshot(L,i,seen,0,visits));}
 return hlConfig(L);
}
'''
MAIN = r'''
int main(int argc,char**argv){
 if(argc!=3)return 2;
 CConfigManager mgr;mgr.dynamic=std::string(argv[1])=="1";mgr.failBorder=std::string(argv[2])=="1";
 auto*L=luaL_newstate();luaL_openlibs(L);
 lua_pushinteger(L,0);lua_setglobal(L,"__pairs_calls");
 lua_newtable(L);lua_pushlightuserdata(L,&mgr);lua_pushcclosure(L,observeConfig,1);lua_setfield(L,-2,"config");lua_setglobal(L,"hl");
 std::string source((std::istreambuf_iterator<char>(std::cin)),std::istreambuf_iterator<char>());
 if(luaL_dostring(L,source.c_str())!=LUA_OK){std::cerr<<lua_tostring(L,-1);return 3;}
 std::cout<<"{\"arguments\":[";bool comma=false;for(const auto&a:arguments){if(comma)std::cout<<",";comma=true;std::cout<<a;}
 std::cout<<"],\"delegations\":[";comma=false;
 for(const auto&c:mgr.calls){if(comma)std::cout<<",";comma=true;std::cout<<"[\""<<hex(c.key)<<"\",\""<<c.type<<"\",\""<<c.data<<"\"]";}
 std::cout<<"],\"errors_hex\":[";comma=false;for(const auto&e:mgr.errors){if(comma)std::cout<<",";comma=true;std::cout<<"\""<<hex(e)<<"\"";}
 lua_getglobal(L,"__pairs_calls");std::cout<<"],\"pairs_calls\":"<<lua_tointeger(L,-1)<<",\"refresh_requests\":"<<Supplementary::r.requests<<"}\n";
 lua_close(L);
}
'''


def function(source):
    start = source.index('static int hlConfig(lua_State* L)')
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end] + '\n'


def fetch(row):
    with urllib.request.urlopen(row['source'], timeout=30) as response:
        body = response.read()
    if hashlib.sha256(body).hexdigest() != row['sha256']:
        raise RuntimeError('Pinned hl.config handler file changed')
    return row['version'], function(body.decode())


def child_limits():
    resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
    resource.setrlimit(resource.RLIMIT_STACK, (1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def canonical(value):
    if value['type'] == 'table':
        rows = [[canonical(k), canonical(v)] for k, v in value['value']]
        rows.sort(key=lambda row: json.dumps(row[0], sort_keys=True))
        value['value'] = rows
    return value


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    contract = json.loads((ROOT / 'docs/hyprland-implementation-contracts.json').read_text())
    api = next(f for f in contract['features'] if f['id'] == 'api:hl.config')
    rows = []
    for version, handler in api['handler_versions'].items():
        url = handler['source'].split('#')[0].replace('github.com/', 'raw.githubusercontent.com/').replace('/blob/', '/')
        rows.append({'version': version, 'source': url, 'sha256': handler['file_sha256']})
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        functions = dict(pool.map(fetch, rows))
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
    observations = []
    with tempfile.TemporaryDirectory(prefix='hye-config-table-') as folder:
        root = Path(folder)
        for row in rows:
            version = row['version']
            cpp = root / f'{version}.cpp'
            binary = root / version
            cpp.write_text(HEADER + functions[version] + SNAPSHOT + MAIN)
            subprocess.run(['c++', '-std=c++23', str(cpp), *flags, '-o', str(binary)], check=True, timeout=90)
            for name, code in CASES:
                dynamic = name != 'static_parse_no_refresh'
                fail = name == 'failed_parse_continues'
                output = subprocess.run([str(binary), str(int(dynamic)), str(int(fail))], input=code, text=True, capture_output=True, check=True, timeout=5, preexec_fn=child_limits)
                result = json.loads(output.stdout)
                result['arguments'] = [canonical(v) for v in result['arguments']]
                # The source's lua_next order is intentionally NOT certified.
                result['delegations'].sort()
                result['errors_hex'].sort()
                observations.append({'version': version, 'name': name, 'lua': code, 'dynamic': dynamic, 'inert_parser_failure': fail, **result})
            print(f'{version}: {len(CASES)} exact hl.config traversal observations', flush=True)
    result = {'format': 'hye-config-table-fixtures-1', 'source_files': rows,
              'evidence_scope': 'Original hlConfig C-API traversal with inert registry/value-parser/refresh/source-location adapters. Delegation only; no scalar acceptance, real runtime state, portable iteration order, ownership or effect certification.',
              'cases': observations}
    encoded = json.dumps(result, indent=2) + '\n'
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != encoded:
            raise SystemExit('hl.config traversal observations differ')
    else:
        OUTPUT.write_text(encoded)


if __name__ == '__main__':
    main()
