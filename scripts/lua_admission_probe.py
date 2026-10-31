#!/usr/bin/env python3
"""Check exact release admission before version/constructor calls.

Official source archives and builds live only in a temporary scope. The fixed
instrumented harness never constructs a Lua state or executes imported Lua.
"""
import argparse
import hashlib
import json
import subprocess
import tempfile
import urllib.request
from pathlib import Path
import workspace_target_profile_probe as target

ROOT = Path(__file__).resolve().parents[1]
CPP = r'''
#include <lua.hpp>
#include <cassert>
#include <cstring>
#include <iostream>
#if LUA_VERSION_NUM == 504
#define API_(name) hye_lua54_##name
#else
#define API_(name) hye_lua55_##name
#endif
static unsigned constructors=0,versions=0;
extern "C" lua_State* __wrap_lua_newstate(lua_Alloc,void*,...){++constructors;return nullptr;}
extern "C" lua_Number __wrap_lua_version(lua_State*){++versions;return LUA_VERSION_NUM;}
extern "C" int API_(runtime_matches_header)();
extern "C" int API_(compile)(const unsigned char*,size_t,const char*,unsigned char*,size_t,size_t*);
extern "C" int API_(subtract_probe)(unsigned char*,size_t,size_t*);
extern "C" void* API_(coercions_new)();
int main(int argc,char**argv){
 assert(argc==2);const bool expected=std::strcmp(argv[1],"match")==0;
 assert(API_(runtime_matches_header)()==expected);
 unsigned char output[4096];size_t written=99;const unsigned char source[]="return 1";
 assert(API_(compile)(source,sizeof(source)-1,"@fixed",output,sizeof(output),&written)==2);assert(written==0);
 written=99;assert(API_(subtract_probe)(output,sizeof(output),&written)==2);assert(written==0);
 assert(API_(coercions_new)()==nullptr);
 assert(constructors==(expected?3u:0u));assert(versions==(expected?4u:0u));
 std::cout<<"{\"constructors\":"<<constructors<<",\"version_calls\":"<<versions<<"}\n";
}
'''


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--observation',type=Path)
    args=parser.parse_args()
    releases={p['release']:p['archive_sha256'] for p in json.loads((ROOT/'data/lua-profiles.json').read_text())['profiles']}
    releases.update(target.RELEASES)
    observations=[]
    with tempfile.TemporaryDirectory(prefix='hye-lua-admission-') as folder:
        root=Path(folder)
        sources={}
        for release,digest in releases.items():
            with urllib.request.urlopen(f'https://www.lua.org/ftp/lua-{release}.tar.gz',timeout=30) as reply:
                data=reply.read(1024*1024+1)
            if len(data)>1024*1024 or hashlib.sha256(data).hexdigest()!=digest:
                raise RuntimeError('Lua admission archive differs')
            target.unpack(data,root,release)
            source=root/f'lua-{release}'/'src';sources[release]=source
            subprocess.run(['make','-C',str(source),'-j2','liblua.a','MYCFLAGS=-fPIC -DLUA_USE_LINUX'],check=True,stdout=subprocess.DEVNULL,timeout=90)
        harness=root/'harness.cpp';harness.write_text(CPP)
        for header,source in sources.items():
            bridge=ROOT/('crates/editor-core/native/lua_syntax.cpp' if header.startswith('5.4.') else 'crates/editor-lua55/native/lua55.cpp')
            objects=[]
            for name,file in [('bridge',bridge),('harness',harness)]:
                obj=root/f'{header}-{name}.o';objects.append(str(obj))
                subprocess.run(['c++','-std=c++17','-I'+str(source),'-c',str(file),'-o',str(obj)],check=True,timeout=60)
            for library,libsource in sources.items():
                executable=root/'probe'
                subprocess.run(['c++','-fuse-ld=bfd',*objects,'-Wl,--wrap=lua_newstate','-Wl,--wrap=lua_version',str(libsource/'liblua.a'),'-lm','-ldl','-o',str(executable)],check=True,timeout=30)
                reply=subprocess.check_output([str(executable),'match' if header==library else 'mismatch'],text=True,env={},timeout=4)
                observations.append({'header_release':header,'library_release':library,'archive_sha256':releases[library],**json.loads(reply)})
            print(f'{header}: exact match and all mismatched linked releases checked before constructors',flush=True)
    result={'format':'hye-lua-release-admission-1','scope':'Six official releases, 36 header/library pairs; instrumented version/constructor calls. No state creation, imported execution, full ABI/build conformance or security certificate. Temporary sources/builds removed.','observations':observations,'source_inputs':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['crates/editor-core/native/lua_syntax.cpp','crates/editor-lua55/native/lua55.cpp','crates/editor-lua55/native/bridge.inc','scripts/lua_admission_probe.py']}}
    if args.observation:args.observation.write_text(json.dumps(result,indent=2)+'\n')
    print('Verified 6 matching and 30 rejected mismatched release pairs; sources/builds removed',flush=True)


if __name__=='__main__':main()
