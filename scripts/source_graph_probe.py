#!/usr/bin/env python3
"""Observe pinned path selection using private files; never execute configuration Lua."""
import argparse
import concurrent.futures
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = json.loads((ROOT / 'data/source-graph-semantics.json').read_text())


def block(source, signature):
    start = source.index(signature)
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end] + '\n'


def fetch(row):
    cache = ROOT / '.dev/sourcegraph-reference' / row['cache']
    if cache.exists():
        body = cache.read_bytes()
    else:
        with urllib.request.urlopen(
            f"https://raw.githubusercontent.com/hyprwm/Hyprland/{row['commit']}/{row['path']}", timeout=30
        ) as response:
            body = response.read()
    blob = hashlib.sha1(f'blob {len(body)}\0'.encode() + body).hexdigest()
    if blob != row['git_blob'] or hashlib.sha256(body).hexdigest() != row['sha256']:
        raise RuntimeError(f"Immutable source mismatch: {row['version']} {row['path']}")
    return (row['version'], Path(row['path']).name), body.decode()


HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <cstdlib>
#include <expected>
#include <filesystem>
#include <format>
#include <glob.h>
#include <iomanip>
#include <iostream>
#include <optional>
#include <ranges>
#include <string>
#include <string_view>
#include <vector>
class CConfigManager {
 public:
 std::string main;
 std::vector<std::string> m_configPaths;
 std::string getMainConfigPath(){return main;}
};
std::vector<std::string> attempted;
auto fixtureStatus(const std::string& path,std::error_code& error){attempted.push_back(path);return std::filesystem::status(path,error);}
std::string q(const std::string&s){std::ostringstream out;out<<std::quoted(s);return out.str();}
void searchObservations(const std::string& main,const std::string& name){
 auto* L=luaL_newstate();luaL_openlibs(L);
 std::cout<<"[";bool comma=false;
 for(const std::string extra : {std::string(""),std::string("/not-present/?.lua;;/other/?/?.lua;"),std::string(";/before/?.lua")}){
  if(comma)std::cout<<",";comma=true;
  const auto directory=std::filesystem::path(main).parent_path();
  const auto path=(directory/"?.lua").string()+";"+(directory/"?/init.lua").string()+(extra.empty()?"":";"+extra);
  lua_getglobal(L,"package");lua_getfield(L,-1,"searchpath");lua_pushstring(L,name.c_str());lua_pushstring(L,path.c_str());
  if(lua_pcall(L,2,2,0)!=LUA_OK)std::abort();
  std::cout<<"{\"inherited\":"<<q(extra)<<",\"selected\":";
  if(lua_isstring(L,-2)){std::cout<<q(lua_tostring(L,-2))<<",\"missing_candidates\":null}";}
  else{
   std::string errors=lua_tostring(L,-1);std::vector<std::string> paths;const std::string marker="no file '";size_t pos=0;
   while((pos=errors.find(marker,pos))!=std::string::npos){pos+=marker.size();const auto end=errors.find('\'',pos);if(end==std::string::npos)std::abort();paths.emplace_back(errors.substr(pos,end-pos));pos=end+1;}
   std::cout<<"null,\"missing_candidates\":[";bool item=false;for(const auto& s:paths){if(item)std::cout<<",";item=true;std::cout<<q(s);}std::cout<<"]}";
  }
  lua_settop(L,0);
 }
 std::cout<<"]";lua_close(L);
}
void array(const std::vector<std::string>&v){std::cout<<"[";bool comma=false;for(auto&s:v){if(comma)std::cout<<",";comma=true;std::cout<<q(s);}std::cout<<"]";}
'''
MAIN = r'''
int main(int argc,char**argv){
 if(argc!=2)return 2;
 const std::string root=argv[1];
 CConfigManager mgr;mgr.main=root+"/config/hyprland.lua";
 std::string name;
 while(std::getline(std::cin,name)){
  const auto separator=name.find('\t');
  mgr.main=root+(separator==std::string::npos?"/config/hyprland.lua":name.substr(separator+1));
  if(separator!=std::string::npos)name.resize(separator);
  mgr.m_configPaths.clear();attempted.clear();
  bool explicitPath=isExplicitRequirePath(name),wildcard=hasGlobMeta(name);
  std::cout<<"{\"main\":"<<q(mgr.main)<<",\"explicit\":"<<(explicitPath?"true":"false")<<",\"wildcard\":"<<(wildcard?"true":"false");
  if(explicitPath){
   std::cout<<",\"base\":"<<q(resolveRequirePath(&mgr,name));
   if(wildcard){auto paths=expandRequireWildcard(&mgr,name);std::cout<<",\"matches\":";if(paths)array(*paths);else std::cout<<"null";std::cout<<",\"error\":"<<(paths?"null":q(paths.error()));}
   else {auto path=resolveExplicitLuaRequireFile(&mgr,name);std::cout<<",\"selected\":"<<(path?q(*path):"null");}
  }
  std::cout<<",\"search_cases\":";searchObservations(mgr.main,name);std::cout<<",\"attempted\":";array(attempted);std::cout<<",\"watched\":";array(mgr.m_configPaths);std::cout<<"}\n";
 }
}
'''
CASES = [
    'colors', 'colors.lua', 'colors.theme', 'nested/colors', '', '.', '..', '~someone/config',
    './exact', './choice', './suffixed.lua', './directory', './missing', './directory/',
    '../shared', './nested/../exact', './nested//color.lua', '~/theme', '~/theme.lua',
    './parts/*.lua', './parts/*', './parts/?.lua', './parts/[ab].lua', './empty/*.lua',
    './absent/*.lua', './directories/*', './parts/../parts/*.lua', './link/*.lua',
    './escape/../target.lua', './parts/[', './parts/.hidden.lua',
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    required = [r for r in MANIFEST['source_files'] if Path(r['path']).name in ['ConfigManager.cpp', 'MiscFunctions.cpp']]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        sources = dict(pool.map(fetch, required))
    observations = []
    with tempfile.TemporaryDirectory(prefix='hye-module-probe-') as folder:
        root = Path(folder)
        for name in ['config/parts', 'config/empty', 'config/nested', 'config/directory', 'config/directories/sub', 'home', 'outside/child']:
            (root / name).mkdir(parents=True, exist_ok=True)
        for name in ['config/hyprland.lua', 'config/exact', 'config/exact.lua', 'config/choice.lua', 'config/suffixed.lua', 'config/suffixed.lua.lua', 'config/directory/init.lua', 'shared.lua', 'config/nested/color.lua', 'home/theme.lua', 'config/parts/a.lua', 'config/parts/b.lua', 'config/parts/aa.lua', 'config/parts/no-extension', 'config/parts/.hidden.lua', 'outside/target.lua', 'config/target.lua']:
            (root / name).touch()
        (root / 'config/link').symlink_to('parts', target_is_directory=True)
        (root / 'config/parts/alias.lua').symlink_to('a.lua')
        (root / 'config/parts/broken.lua').symlink_to('missing.lua')
        (root / 'config/escape').symlink_to('../outside/child', target_is_directory=True)
        names = CASES + [str(root / 'shared'), str(root / 'config/parts/*.lua')]
        for main_path in ['/config//hyprland.lua', '/config/./hyprland.lua', '/config/nested/../hyprland.lua', '/main.lua', '/config/nested//../hyprland.lua']:
            names.extend([name+'\t'+main_path for name in ['theme', './exact']])
        for version in ['baseline', 'current']:
            manager = sources[version, 'ConfigManager.cpp']
            cpp = HEADER + block(sources[version, 'MiscFunctions.cpp'], 'std::string absolutePath')
            for signature in ['static std::string normalizedConfigPath', 'static void trackConfigPath', 'static bool isExplicitRequirePath', 'static bool hasGlobMeta', 'static std::string resolveRequirePath', 'static std::optional<std::string> resolveExplicitLuaRequireFile', 'static void trackWildcardParentDirectory', 'static std::expected<std::vector<std::string>, std::string> expandRequireWildcard']:
                cpp += block(manager, signature).replace("std::filesystem::status(", "fixtureStatus(")
            path = root / 'probe.cpp'
            path.write_text(cpp + MAIN)
            binary = root / 'probe'
            subprocess.run(['c++', '-std=c++23', '-O0', str(path), *shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True)), '-o', str(binary)], check=True, timeout=45)
            env = dict(os.environ, HOME=str(root / 'home'))
            result = subprocess.run([str(binary), str(root)], input='\n'.join(names)+'\n', text=True, capture_output=True, check=True, timeout=4, env=env, cwd=root)
            lines = result.stdout.replace(str(root), '<root>').splitlines()
            if len(lines) != len(names):
                raise RuntimeError('Source probe result count differs')
            observations.extend({'version': version, 'module': name.split('\t')[0].replace(str(root), '<root>'), **json.loads(line)} for name, line in zip(names, lines))
            print(f'{version}: {len(lines)} actual module-path/wildcard observations')
    content = json.dumps({'format': 'hye-module-path-fixtures-1', 'meaning': 'Actual explicit-path/candidate/glob/watcher-prefix functions and absolutePath helper over private regular files, directories and symlinks. A minimal ConfigManager supplies only main path and tracked-path storage. A status adapter records queries and forwards to the original filesystem status implementation. Actual Lua 5.4 package.searchpath is queried directly through its C API with private candidate paths; no config Lua is executed. No package cache/custom-searcher, file-race, authorization, watcher-delivery or compositor certificate.', 'cases': observations}, indent=2, sort_keys=True) + '\n'
    target = ROOT / 'data/source-graph-fixtures.json'
    if args.check:
        if target.read_text() != content:
            raise RuntimeError('Module-path observations differ')
    else:
        target.write_text(content)


if __name__ == '__main__':
    main()
