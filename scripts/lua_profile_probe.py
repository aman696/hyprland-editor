"""Verify each recorded Lua release against all ordered source observations.

Only trusted synthetic fixtures execute. Official archive bytes are hash checked;
all downloaded sources and build products are removed with the temporary scope.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

sys.dont_write_bytecode = True
import ordered_module_probe as ordered

ROOT = ordered.ROOT

BRIDGE_PROBE = r'''
#include <lua.hpp>
#include <cassert>
#include <cstring>
#include <cstdint>
#include <cmath>
#include <limits>
extern "C" unsigned int hye_lua54_release();
extern "C" size_t hye_lua54_identity(unsigned char*, size_t);
extern "C" int hye_lua54_compile(const unsigned char*, size_t, const char*, unsigned char*, size_t, size_t*);
struct Converted {unsigned int flags;int64_t integer;double number;size_t length;int64_t coerced_integer;};
extern "C" void* hye_lua54_coercions_new();
extern "C" int hye_lua54_numeric_locale_c();
extern "C" void hye_lua54_coercions_close(void*);
extern "C" int hye_lua54_convert(void*,int,int64_t,double,const unsigned char*,size_t,Converted*,unsigned char*,size_t);
int main() {
    assert(hye_lua54_release() == LUA_VERSION_RELEASE_NUM);
    unsigned char identity[512] = {};
    const auto length = hye_lua54_identity(identity, sizeof(identity));
    assert(length > 0 && length < sizeof(identity));
    assert(std::strstr(reinterpret_cast<char*>(identity), LUA_RELEASE " "));
    assert(hye_lua54_identity(identity, 1) == 0);
    unsigned char error[4096];
    size_t written = 0;
    const char* good = "os.execute('must never execute'); return 1";
    assert(hye_lua54_compile(reinterpret_cast<const unsigned char*>(good), std::strlen(good), "@profile", error, sizeof(error), &written) == 0);
    const char* bad = "local x <const> = 1; x = 2";
    assert(hye_lua54_compile(reinterpret_cast<const unsigned char*>(bad), std::strlen(bad), "@profile", error, sizeof(error), &written) == 1);
    assert(written > 0 && written <= sizeof(error));
    const unsigned char binary[] = {27, 'L', 'u', 'a'};
    assert(hye_lua54_compile(binary, sizeof(binary), "@profile", error, sizeof(error), &written) == 1);
    assert(hye_lua54_numeric_locale_c()==1);
    auto* state=hye_lua54_coercions_new();assert(state);
    Converted out{};unsigned char text[128];
    const unsigned char numeric[]=" 0x1p0 ";
    assert(hye_lua54_convert(state,4,0,0,numeric,sizeof(numeric)-1,&out,text,sizeof(text))==0);
    assert((out.flags&2)&&!(out.flags&1)&&out.number==1&&out.coerced_integer==1);
    assert(hye_lua54_convert(state,3,0,3.0,nullptr,0,&out,text,sizeof(text))==0);
    assert(!(out.flags&1)&&(out.flags&4)&&out.length==3&&std::memcmp(text,"3.0",3)==0);
    assert(hye_lua54_convert(state,3,0,3.5,nullptr,0,&out,text,sizeof(text))==0);
    assert(out.coerced_integer==0); // lua_tointeger rejects the fractional value.
    assert(hye_lua54_convert(state,2,std::numeric_limits<int64_t>::max(),0,nullptr,0,&out,text,sizeof(text))==0);
    assert(out.coerced_integer==std::numeric_limits<int64_t>::max());
    const unsigned char nul[]={'a',0,'b'};
    assert(hye_lua54_convert(state,4,0,0,nul,sizeof(nul),&out,text,sizeof(text))==0);
    assert(out.length==1&&text[0]=='a');
    for(int i=0;i<4096;++i){assert(hye_lua54_convert(state,2,i,0,nullptr,0,&out,text,sizeof(text))==0);assert(out.integer==i&&(out.flags&1));}
    assert(hye_lua54_convert(state,4,0,0,nul,65537,&out,text,sizeof(text))==3);
    assert(hye_lua54_convert(state,1,0,0,nullptr,0,&out,text,sizeof(text))==0);
    assert(out.flags==8&&out.integer==0);
    hye_lua54_coercions_close(state);
    // Trusted literal-printer reference only: no libraries or user source are loaded.
    auto* literals=luaL_newstate();assert(literals);
    const char* code=R"lua(return -0.0,3.0,(-9223372036854775807 - 1),"\";os.execute('never') --\010\\é")lua";
    assert(luaL_loadbufferx(literals,code,std::strlen(code),"@trusted-printers","t")==LUA_OK);
    assert(lua_pcall(literals,0,4,0)==LUA_OK);
    assert(!lua_isinteger(literals,1)&&std::signbit(lua_tonumber(literals,1))&&lua_tonumber(literals,1)==0);
    assert(!lua_isinteger(literals,2)&&lua_tonumber(literals,2)==3.0);
    assert(lua_isinteger(literals,3)&&lua_tointeger(literals,3)==std::numeric_limits<lua_Integer>::min());
    const char* wanted="\";os.execute('never') --\n\\é";size_t size=0;
    const char* result=lua_tolstring(literals,4,&size);assert(size==std::strlen(wanted)&&std::memcmp(result,wanted,size)==0);
    lua_close(literals);
}
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--release', action='append')
    args = parser.parse_args()
    matrix = json.loads((ROOT/'data/lua-profiles.json').read_text())
    base_bytes = (ROOT/'data/ordered-module-fixtures.json').read_bytes()
    if hashlib.sha256(base_bytes).hexdigest() != matrix['canonical_fixtures_sha256']:
        raise RuntimeError('Lua profiles must be reverified after changing canonical observations')
    if hashlib.sha256((ROOT/'data/runtime-path-fixtures.json').read_bytes()).hexdigest() != matrix['runtime_path_fixtures_sha256']:
        raise RuntimeError('Lua profiles must be reverified after changing runtime path observations')
    if hashlib.sha256((ROOT/'data/scalar-value-fixtures.json').read_bytes()).hexdigest() != matrix['scalar_value_fixtures_sha256']:
        raise RuntimeError('Lua profiles must be reverified after changing scalar observations')
    base = json.loads(base_bytes)
    if args.release and any(r not in [p['release'] for p in matrix['profiles']] for r in args.release):
        raise RuntimeError('Unknown requested Lua profile')
    for profile in matrix['profiles']:
        if profile.get('primitive_conversion_profile') != 'hye-primitive-coercions-1':
            raise RuntimeError('Unknown primitive conversion profile')
        release = profile['release']
        if args.release and release not in args.release:
            continue
        with tempfile.TemporaryDirectory(prefix='hye-lua-profile-') as folder:
            temp = Path(folder)
            with urllib.request.urlopen(profile['archive_url'], timeout=30) as reply:
                archive_bytes = reply.read(1024*1024)
            if hashlib.sha256(archive_bytes).hexdigest() != profile['archive_sha256']:
                raise RuntimeError(f'Official Lua {release} archive hash differs')
            with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode='r:gz') as archive:
                archive.extractall(temp, filter='data')
            source = temp/('lua-'+release)/'src'
            subprocess.run(['make','-C',str(source),'-j2','liblua.a','MYCFLAGS=-fPIC -DLUA_USE_LINUX'],check=True,stdout=subprocess.DEVNULL,timeout=60)
            bridge_source = temp/'bridge.cpp'
            bridge_source.write_text(BRIDGE_PROBE)
            bridge = temp/'bridge'
            subprocess.run(['g++','-std=c++17',f'-I{source}',str(ROOT/'crates/editor-core/native/lua_syntax.cpp'),str(bridge_source),str(source/'liblua.a'),'-lm','-ldl','-o',str(bridge)],check=True,timeout=30)
            subprocess.run([str(bridge)],check=True,timeout=5)
            (temp/'lua5.4.pc').write_text(f'Name: Lua\nDescription: private hash-pinned reference\nVersion: {release}\nCflags: -I{source}\nLibs: {source}/liblua.a -lm -ldl\n')
            output = temp/'observations.json'
            subprocess.run([sys.executable,str(ROOT/'scripts/ordered_module_probe.py'),'--output',str(output)],env=dict(os.environ,PKG_CONFIG_LIBDIR=str(temp)),check=True,timeout=90,cwd=ROOT)
            actual_bytes = output.read_bytes()
            if hashlib.sha256(actual_bytes).hexdigest() != profile['observations_sha256'] or json.loads(actual_bytes) != ordered.materialize(base,profile):
                raise RuntimeError(f'Lua {release} ordered observations differ from its recorded profile')
            subprocess.run([sys.executable,str(ROOT/'scripts/runtime_path_probe.py'),'--check'],env=dict(os.environ,PKG_CONFIG_LIBDIR=str(temp)),check=True,timeout=60,cwd=ROOT)
            subprocess.run([sys.executable,str(ROOT/'scripts/scalar_value_probe.py'),'--check'],env=dict(os.environ,PKG_CONFIG_LIBDIR=str(temp)),check=True,timeout=90,cwd=ROOT)
            subprocess.run([sys.executable,str(ROOT/'scripts/complex_scalar_probe.py'),'--check'],env=dict(os.environ,PKG_CONFIG_LIBDIR=str(temp),TMPDIR=str(temp)),check=True,timeout=120,cwd=ROOT)
            print(f'Lua {release}: compiler bridge and {len(base["cases"])} ordered observations verified; error line uses {profile["error_call_line"]}',flush=True)


if __name__ == '__main__':
    main()
