"""Observe current Lua search templates and pinned lexical normalization on private files.

No configuration Lua is executed. Upstream originals/builds are temporary.
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
import source_graph_probe as paths
ROOT = paths.ROOT
NORMAL_PATHS = ['', '.', './', '..', '../', '../.', 'a/.', 'a/..', 'a/b/..', 'a/b/../', 'a/../../b', 'a/../../', '/', '//', '///a//b/', '/a/../../b', '/a/../', '/a/./b/.', '/a/link/../b.lua', '/a/..//b/../.', '/..', '/a/../..', 'a//b.lua', './a.lua', 'a/../b.lua']
MODULES = ['', '.', '..', 'colors', 'colors.theme', './colors.lua', 'nested/colors', '?']
TEMPLATES = ['', ';', ';;', 'x;;y;', ';?;', '?;?.lua', 'fixed.lua', '?/?/?.lua', '<root>/config/?.lua;<root>/config/?/init.lua', '<root>/missing/?.lua;<root>/vendor/?.lua']
MAIN = r'''
int main(){auto* L=luaL_newstate();luaL_openlibs(L);std::string line;
 while(std::getline(std::cin,line)){
  if(line.starts_with("N\t")){auto input=line.substr(2);std::cout<<"{\"path\":"<<q(input)<<",\"normalized\":"<<q(normalizedConfigPath(input))<<"}\n";continue;}
  auto pos=line.find('\t');auto name=line.substr(0,pos),path=line.substr(pos+1);
  lua_getglobal(L,"package");lua_getfield(L,-1,"searchpath");lua_pushlstring(L,name.data(),name.size());lua_pushlstring(L,path.data(),path.size());
  if(lua_pcall(L,2,2,0)!=LUA_OK)return 2;
  std::cout<<"{\"module\":"<<q(name)<<",\"package_path\":"<<q(path)<<",\"selected\":";
  if(lua_isstring(L,-2))std::cout<<q(lua_tostring(L,-2))<<",\"missing_candidates\":null}";
  else{std::cout<<"null,\"missing_candidates\":[";std::string errors=lua_tostring(L,-1);size_t start=0;bool comma=false;while((start=errors.find("no file '",start))!=std::string::npos){start+=9;auto end=errors.find('\'',start);if(end==std::string::npos)return 3;if(comma)std::cout<<",";comma=true;std::cout<<q(errors.substr(start,end-start));start=end+1;}std::cout<<"]}";}
  std::cout<<"\n";lua_settop(L,0);
 }lua_close(L);
}
'''
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    rows=[r for r in paths.MANIFEST['source_files'] if Path(r['path']).name=='ConfigManager.cpp']
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        source=dict(pool.map(paths.fetch,rows))
    functions=[paths.block(source[v,'ConfigManager.cpp'],'static std::string normalizedConfigPath') for v in ['baseline','current']]
    if functions[0]!=functions[1]:raise RuntimeError('Versioned normalization helpers diverged')
    with tempfile.TemporaryDirectory(prefix='hye-current-path-') as folder:
        root=Path(folder);(root/'config').mkdir();(root/'vendor').mkdir()
        (root/'vendor/colors.lua').write_text('-- inert bytes')
        (root/'config/colors.lua').write_text('-- inert bytes')
        # Lua searchpath uses readability, not regular-file status: a directory
        # may be selected, then fail later during actual loading.
        (root/'config/directory.lua').mkdir()
        (root/'config/unreadable.lua').write_text('-- inert bytes');(root/'config/unreadable.lua').chmod(0)
        cases=[f'{name}\t{path}'.replace('<root>',str(root)) for name in MODULES for path in TEMPLATES]
        cases += [f'{name}\t{root}/config/?.lua;{root}/vendor/?.lua' for name in ['colors','directory','unreadable']]
        requests=cases+['N\t'+p for p in NORMAL_PATHS]
        cpp=root/'probe.cpp';cpp.write_text(paths.HEADER+functions[0]+MAIN)
        flags=shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','lua5.4'],text=True))
        binary=root/'probe';subprocess.run(['c++','-std=c++23',str(cpp),*flags,'-o',str(binary)],check=True,timeout=45)
        result=subprocess.run([str(binary)],input='\n'.join(requests)+'\n',text=True,capture_output=True,check=True,cwd=root,timeout=5)
        records=[json.loads(line.replace(str(root),'<root>')) for line in result.stdout.splitlines()]
        if len(records)!=len(requests):raise RuntimeError('Runtime path observation count differs')
    value={'format':'hye-runtime-path-fixtures-1','audit_manifest_sha256':hashlib.sha256((ROOT/'data/source-graph-semantics.json').read_bytes()).hexdigest(),'meaning':'Direct system Lua 5.4 package.searchpath C API and identical hash-pinned normalization helpers from both audited snapshots. Private files/directories only; no config execution, actual module loading, authorization, race, ABI or compositor certificate.','search':records[:len(cases)],'normalization':records[len(cases):]}
    target=ROOT/'data/runtime-path-fixtures.json';content=json.dumps(value,indent=2,sort_keys=True)+'\n'
    if args.check:
        if target.read_text()!=content:raise RuntimeError('Current search-path observations differ')
    else:target.write_text(content)
    print(f'{len(value["search"])} current search-path and {len(value["normalization"])} lexical normalization observations')
if __name__=='__main__':main()
