#!/usr/bin/env python3
"""Pinned require/cache/searcher functions over private, trusted synthetic Lua fixtures."""
import argparse
import concurrent.futures
import json
import re
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import source_graph_probe as paths

ROOT = paths.ROOT
HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <cstdlib>
#include <expected>
#include <filesystem>
#include <format>
#include <iomanip>
#include <iostream>
#include <optional>
#include <ranges>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>
#include <glob.h>
template<typename T,typename U>T sc(U value){return static_cast<T>(value);}
class CConfigManager {
 public:
 std::string main;std::vector<std::string> m_configPaths,errors;
 std::string getMainConfigPath(){return main;}
 void addError(const std::string& error){errors.push_back(error);}
 static CConfigManager* fromLuaState(lua_State* L){lua_getfield(L,LUA_REGISTRYINDEX,"hl_lua_manager");auto* p=static_cast<CConfigManager*>(lua_touserdata(L,-1));lua_pop(L,1);return p;}
};
std::string q(const std::string& value){std::ostringstream out;out<<'"';for(unsigned char c:value){switch(c){case '"':out<<"\\\"";break;case '\\':out<<"\\\\";break;case '\n':out<<"\\n";break;case '\r':out<<"\\r";break;case '\t':out<<"\\t";break;default:if(c<32)out<<"\\u"<<std::hex<<std::setw(4)<<std::setfill('0')<<static_cast<int>(c)<<std::dec;else out<<c;}}out<<'"';return out.str();}
void json(lua_State* L,int index,int depth=0){
 index=lua_absindex(L,index);if(depth>12){std::cout<<"null";return;}
 switch(lua_type(L,index)){
 case LUA_TNIL:std::cout<<"null";break;
 case LUA_TBOOLEAN:std::cout<<(lua_toboolean(L,index)?"true":"false");break;
 case LUA_TNUMBER:if(lua_isinteger(L,index))std::cout<<lua_tointeger(L,index);else std::cout<<std::setprecision(17)<<lua_tonumber(L,index);break;
 case LUA_TSTRING:{size_t n;const char* s=lua_tolstring(L,index,&n);std::cout<<q(std::string(s,n));break;}
 case LUA_TTABLE:{auto n=lua_rawlen(L,index);bool array=n>0;lua_pushnil(L);while(lua_next(L,index)){array=array&&lua_isinteger(L,-2)&&lua_tointeger(L,-2)>=1&&static_cast<size_t>(lua_tointeger(L,-2))<=n;lua_pop(L,1);}
 bool comma=false;std::cout<<(array?"[":"{");if(array){for(size_t i=1;i<=n;++i){if(comma)std::cout<<",";comma=true;lua_rawgeti(L,index,i);json(L,-1,depth+1);lua_pop(L,1);}}
 else{lua_pushnil(L);while(lua_next(L,index)){if(comma)std::cout<<",";comma=true;if(lua_type(L,-2)==LUA_TSTRING)std::cout<<q(lua_tostring(L,-2));else if(lua_isinteger(L,-2))std::cout<<q(std::to_string(lua_tointeger(L,-2)));else std::cout<<q("<non-string-key>");std::cout<<":";json(L,-1,depth+1);lua_pop(L,1);}}std::cout<<(array?"]":"}");break;}
 default:std::cout<<q(lua_typename(L,lua_type(L,index)));
 }
}
bool standard(std::string_view name){for(auto s:{"_G","coroutine","debug","io","math","os","package","string","table","utf8","bit32","jit"})if(name==s)return true;return false;}
void loaded(lua_State* L,int index){index=lua_absindex(L,index);std::cout<<"{";bool comma=false;if(lua_istable(L,index)){lua_pushnil(L);while(lua_next(L,index)){if(lua_type(L,-2)==LUA_TSTRING&&!standard(lua_tostring(L,-2))){if(comma)std::cout<<",";comma=true;std::cout<<q(lua_tostring(L,-2))<<":";json(L,-1);}lua_pop(L,1);}}std::cout<<"}";}
'''
MAIN = r'''
int main(int argc,char**argv){if(argc!=2)return 2;std::string root=argv[1],line;
 while(std::getline(std::cin,line)){
  auto* L=luaL_newstate();luaL_openlibs(L);CConfigManager mgr;mgr.main=root+"/config/hyprland.lua";
  lua_pushlightuserdata(L,&mgr);lua_setfield(L,LUA_REGISTRYINDEX,"hl_lua_manager");
  lua_getglobal(L,"package");lua_pushliteral(L,"");lua_setfield(L,-2,"path");lua_pushliteral(L,"");lua_setfield(L,-2,"cpath");lua_pop(L,1);
  setup(L,&mgr);
  lua_pushlstring(L,root.data(),root.size());lua_setglobal(L,"__fixture_root");
  lua_pushcfunction(L,reloadCache);lua_setglobal(L,"__fixture_reload");
  if(luaL_dostring(L,"__trace={};function emit(v)table.insert(__trace,v)end")!=LUA_OK)return 3;
  auto status=luaL_dostring(L,line.c_str());if(status!=LUA_OK)lua_pop(L,1);
  std::cout<<"{\"ok\":"<<(status==LUA_OK?"true":"false")<<",\"error_count\":"<<mgr.errors.size()<<",\"trace\":[";
  lua_getglobal(L,"__trace");for(size_t i=1;i<=lua_rawlen(L,-1);++i){if(i>1)std::cout<<",";lua_rawgeti(L,-1,i);json(L,-1);lua_pop(L,1);}lua_pop(L,1);
  std::cout<<"],\"watched\":[";for(size_t i=0;i<mgr.m_configPaths.size();++i){if(i)std::cout<<",";std::cout<<q(mgr.m_configPaths[i]);}
  std::cout<<"],\"global_loaded\":";lua_getglobal(L,"package");lua_getfield(L,-1,"loaded");loaded(L,-1);lua_pop(L,2);
  std::cout<<",\"registry_loaded\":";lua_getfield(L,LUA_REGISTRYINDEX,LUA_LOADED_TABLE);loaded(L,-1);lua_pop(L,1);std::cout<<"}\n";lua_close(L);
 }
}
'''
FILES = {
    'literal/*.lua': "emit('literal-star');return 'literal-file'",
    'nilmod.lua': "emit('nilmod');return nil",
    'falsemod.lua': "emit('falsemod');return false",
    'value.lua': "emit('value');return {token='value'}",
    'assigned_false.lua': "emit('assigned_false');package.loaded.assigned_false=false;return nil",
    'assigned_table.lua': "emit('assigned_table');package.loaded.assigned_table={token='assigned'};return nil",
    'assigned/failed.lua': "local name=...;emit('assigned-failed');package.loaded[name]=true;error('assigned failure')",
    'misleading.lua': "emit('misleading');error(\"module 'misleading' not found: fixture\",0)",
    'number.lua': "emit('number');return 7",
    'multiret.lua': "emit('multiret');return 7,8",
    'runtime_error.lua': "emit('runtime_error');error('fixture failure')",
    'table_error.lua': "emit('table_error');error({kind='fixture'})",
    'syntax_error.lua': "local value = ",
    'sibling.lua': "emit('main-sibling');return 'main'",
    'nested/sibling.lua': "emit('nested-sibling');return 'nested'",
    'nested/child.lua': "emit('child');return require('./sibling')",
    'cycle.lua': "__cycle=(__cycle or 0)+1;emit(__cycle);if __cycle<3 then require('cycle')end;return {count=__cycle}",
    'parts/a.lua': "emit('a');return {token='a'}",
    'parts/b.lua': "emit('b');return nil",
    'parts/c.lua': "emit('c');return false",
    'parts/d.lua': "emit('d');error('part failure')",
    'parts/e.lua': "emit('e');return 5",
    'parts/f.lua': "local value = ",
}
CASES = [
    ('nil_cached_as_true', "local a=require('nilmod');local b=require('nilmod');emit({a=a,b=b,same=a==b})"),
    ('false_reexecutes', "emit(require('falsemod'));emit(require('falsemod'))"),
    ('table_identity_cached', "local a=require('value');a.changed=true;local b=require('value');emit({same=a==b,b=b})"),
    ('number_cached', "local a=require('number');local b=require('number');emit({a=a,b=b})"),
    ('module_first_return_only', "emit({require('multiret')});emit({require('multiret')})"),
    ('path_aliases_distinct_keys', "local a=require('value');local b=require('./value');local c=require('./value.lua');emit({ab=a==b,bc=b==c})"),
    ('normalized_path_alias_key', "local a=require('./value');local b=require('./nested/../value');emit(a==b)"),
    ('preloaded_wins_file', "package.preload.value=function()emit('preload');return {token='preload'}end;emit(require('value'))"),
    ('preloaded_explicit_wins_file', "package.preload['./value']=function()emit('preload');return 9 end;emit(require('./value'))"),
    ('runtime_error_becomes_cached_empty_table', "local a=require('runtime_error');local b=require('runtime_error');emit({same=a==b,a=a,b=b})"),
    ('syntax_error_becomes_cached_empty_table', "local a=require('syntax_error');local b=require('syntax_error');emit({same=a==b,a=a})"),
    ('table_error_becomes_empty_table', "emit(require('table_error'))"),
    ('not_found_still_throws', "local ok=pcall(require,'missing');emit(ok);local again=pcall(require,'missing');emit(again)"),
    ('explicit_not_found_still_throws', "emit(pcall(require,'./missing'))"),
    ('wildcard_order_results_and_cached_error', "local a=require('./parts/*.lua');local b=require('./parts/*.lua');emit({same=a==b,items=a})"),
    ('wildcard_and_individual_path_cache', "local a=require('./parts/[ab].lua');emit(require(__fixture_root..'/config/parts/a.lua'));emit(a)"),
    ('wildcard_aliases_reuse_individual_paths', "local a=require('./parts/[ab].lua');local b=require('./parts/../parts/[ab].lua');emit({same=a==b,first=a[1]==b[1]})"),
    ('wildcard_false_individual_reexecutes', "local a=require('./parts/c.lua');local b=require('./parts/[c].lua');emit({a=a,b=b})"),
    ('wildcard_missing_still_throws', "emit(pcall(require,'./no-directory/*.lua'))"),
    ('wildcard_cached_preload_pattern', "package.loaded['./parts/*.lua']={token='pattern'};emit(require('./parts/*.lua'))"),
    ('wildcard_false_pattern_loads', "package.loaded['./parts/[ab].lua']=false;emit(require('./parts/[ab].lua'))"),
    ('false_cache_override_reexecutes', "require('value');package.loaded.value=false;require('value')"),
    ('nil_cache_override_reexecutes', "require('value');package.loaded.value=nil;require('value')"),
    ('truthy_cache_override_skips_file', "package.loaded.value={token='override'};emit(require('value'))"),
    ('package_loaded_rebind_differs_from_registry', "local a=require('value');package.loaded={};local b=require('value');emit(a==b)"),
    ('wildcard_after_loaded_rebind', "local a=require('./parts/[ab].lua');package.loaded={};local b=require('./parts/[ab].lua');emit({same=a==b,first=a[1]==b[1]})"),
    ('nested_explicit_main_relative', "emit(require('./nested/child'))"),
    ('finite_self_recursion_not_loading_sentinel', "emit(require('cycle'))"),
    ('original_require_bypasses_error_fallback', "emit(pcall(__require,'runtime_error'));emit(pcall(__require,'runtime_error'))"),
    ('reload_clears_user_cache_preserves_stdlibs', "local a=require('value');local math_before=package.loaded.math;__fixture_reload();local b=require('value');emit({same=a==b,math_same=math_before==package.loaded.math})"),
    ('reload_after_loaded_rebind', "local a=require('value');package.loaded={};__fixture_reload();local b=require('value');emit(a==b)"),
    ('nil_return_retains_assigned_false', "emit(require('assigned_false'));emit(require('assigned_false'))"),
    ('nil_return_retains_assigned_table', "local a=require('assigned_table');local b=require('assigned_table');emit({same=a==b,a=a})"),
    ('wildcard_error_retains_module_cache_assignment', "emit(require('./assigned/[f]ailed.lua'));emit(require(__fixture_root..'/config/assigned/failed.lua'))"),
    ('safe_error_overwrites_module_cache_assignment', "local a=require('./assigned/failed.lua');local b=require('./assigned/failed.lua');emit({same=a==b,a=a})"),
    ('error_prefix_is_compared_not_resolver_category', "emit(pcall(require,'misleading'));emit(pcall(require,'misleading'))"),
    ('non_table_loaded_wildcard_not_cached', "package.loaded=false;local a=require('./parts/[ab].lua');local b=require('./parts/[ab].lua');emit({same=a==b,first=a[1]==b[1]})"),
    ('non_table_loaded_safe_error_not_cached', "package.loaded=false;require('runtime_error');require('runtime_error')"),
    ('original_require_treats_glob_as_literal_filename', "emit({__require('./literal/*')})"),
    ('safe_wildcard_original_searcher_may_reject_metachar_filename', "emit(require('./literal/*'))"),
    ('reload_numeric_loaded_key_can_fail', "package.loaded[1]=true;emit(pcall(__fixture_reload))"),
    ('reload_numeric_string_key_is_removed', "package.loaded['1']=true;emit(pcall(__fixture_reload));emit(package.loaded['1']==nil)"),
    ('modified_path_for_named_module', "package.path=__fixture_root..'/config/nested/?.lua';emit(require('sibling'))"),
    ('modified_path_does_not_rebase_explicit_module', "package.path=__fixture_root..'/config/nested/?.lua';emit(require('./sibling'))"),
]


def setup(source):
    # Exact source initialization fragments: prefix path, require wrapper, and file-searcher hook.
    begin = source.index('    std::filesystem::path configDir =')
    end = source.index('    Bindings::registerBindings', begin)
    prefix = source[begin:end]
    begin = source.index('    // Hook package.searchers[2]')
    end = source.index('    // hook print function', begin)
    hook = source[begin:end]
    body = (prefix + hook).replace('m_lua', 'L').replace('m_mainConfigPath', 'mgr->main').replace('this', 'mgr')
    return 'void setup(lua_State* L,CConfigManager* mgr){\n'+body+'}\n'


def reload_cache(source):
    begin = source.index('        lua_getglobal(m_lua, "package");', source.index('auto phase1Load ='))
    end = source.index('        if (luaL_loadfile', begin)
    body = source[begin:end].replace('m_lua', 'L')
    return 'int reloadCache(lua_State* L){\n'+body+'return 0;}\n'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    rows = [r for r in paths.MANIFEST['source_files'] if Path(r['path']).name in ['ConfigManager.cpp', 'MiscFunctions.cpp']]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        sources = dict(pool.map(paths.fetch, rows))
    observations = []
    standard_keys = {}
    for version in ['baseline', 'current']:
        fragment = reload_cache(sources[version, 'ConfigManager.cpp'])
        array = re.search(r'STDLIB\[\] = \{([^}]+)\}', fragment).group(1)
        standard_keys[version] = re.findall(r'"([^"]+)"', array)
        if len(standard_keys[version]) != 12:
            raise RuntimeError('Review the source reload preservation list')
    with tempfile.TemporaryDirectory(prefix='hye-loader-probe-') as folder:
        root = Path(folder)
        (root/'home').mkdir()
        for name, code in FILES.items():
            target = root/'config'/name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(code)
        for version in ['baseline', 'current']:
            manager = sources[version, 'ConfigManager.cpp']
            cpp = HEADER + paths.block(sources[version, 'MiscFunctions.cpp'], 'std::string absolutePath')
            for signature in ['static std::string normalizedConfigPath', 'static void trackConfigPath', 'static bool isExplicitRequirePath', 'static bool hasGlobMeta', 'static std::string resolveRequirePath', 'static std::optional<std::string> resolveExplicitLuaRequireFile', 'static void trackWildcardParentDirectory', 'static std::expected<std::vector<std::string>, std::string> expandRequireWildcard', 'static bool pushPackageLoaded', 'static void setPackageLoaded', 'static int requireWildcard', 'static void trackRequiredLuaModulePath', 'static int safeLuaRequire']:
                cpp += paths.block(manager, signature)
            cpp += setup(manager)+reload_cache(manager)+MAIN
            source = root/'probe.cpp';source.write_text(cpp)
            binary = root/'probe'
            flags = shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','lua5.4'], text=True))
            subprocess.run(['c++','-std=c++23','-O0',str(source),*flags,'-o',str(binary)], check=True, timeout=45)
            result = subprocess.run([str(binary),str(root)], input='\n'.join(code for _,code in CASES)+'\n', text=True, capture_output=True, check=True, timeout=6, cwd=root, env={'HOME':str(root/'home')})
            lines = result.stdout.replace(str(root),'<root>').splitlines()
            if len(lines)!=len(CASES):
                raise RuntimeError(f'Loader probe result count differs: {result.stderr}')
            observations.extend({'version':version,'case':name,'source':code,**json.loads(line)} for (name,code),line in zip(CASES,lines))
            print(f'{version}: {len(lines)} actual require/cache/searcher observations')
    content = json.dumps({'format':'hye-module-load-fixtures-1','meaning':'Actual safe require, wildcard, package cache helpers, file-searcher initialization and reload cache-reset fragment with a minimal ConfigManager and real Lua 5.4. Only private trusted synthetic modules execute. Registry/global loaded maps, ordered path tracking, fallback/error counts and traces are observed. No user config execution, authorization, file-race, full reload, compositor, sandbox or dependency ABI certificate.','standard_keys':standard_keys,'module_files':FILES,'cases':observations},indent=2,sort_keys=True)+'\n'
    target=ROOT/'data/module-load-fixtures.json'
    if args.check:
        if target.read_text()!=content:
            raise RuntimeError('Module load observations differ')
    else:
        target.write_text(content)


if __name__=='__main__':
    main()
