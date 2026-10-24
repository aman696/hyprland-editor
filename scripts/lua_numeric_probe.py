#!/usr/bin/env python3
"""Fingerprint fixed non-executing compiler probes and trusted numeric cases.

Only these literal fixtures execute in reference builds. Source archives and
the narrowly patched comparison libraries remain temporary. No user Lua runs.
"""
import argparse
import hashlib
import json
import os
import shutil
import struct
from pathlib import Path
import subprocess
import tempfile
import urllib.request

import workspace_target_profile_probe as target

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/lua-numeric-codegen.json'
FIXTURES = ROOT / 'data/lua-numeric-fixtures.json'
PRELUDE = 'local z=0;local nz=-0.0;local yes=true;local no=false;local t={zero=0};local function zero()return 0 end;'
EXPRESSIONS = [
    '0', '(0)', '((0))', '0x0', '-0', '1-1', '2*0', '3//4', '3%3',
    'z', 't.zero', 'zero()', '(function()return 0 end)()', '0.0', '-0.0',
    'true and 0', 'false or 0', 'nil or 0', '0 and 0', "'' and 0",
    'yes and 0', 'no or 0', '(no or true) and 0', '(yes and false) or 0',
    '(z==0) and 0', '(z==1) or 0', '(not false) and 0', '(not true) or 0',
    '(not no) and 0', '(not yes) or 0', '(z or true) and 0',
    '(false and z) or 0', '(true or z) and 0', '(("a".."b") and 0)', '#""',
    '((1-1) or true) and 0', '((z and false) or true) and 0',
]
SOURCES = [PRELUDE + 'return nz-(' + expression + ')' for expression in EXPRESSIONS]
CPP = r'''
#include <lua.hpp>
#include <cstdint>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <string>
#define CAT_(a,b) a##b
#define CAT(a,b) CAT_(a,b)
#if LUA_VERSION_NUM == 504
#define API(name) CAT(hye_lua54_,name)
#else
#define API(name) CAT(hye_lua55_,name)
#endif
extern "C" int API(subtract_probe)(unsigned char*,size_t,size_t*);
std::string hex(const unsigned char*p,size_t n){const char*d="0123456789abcdef";std::string out;for(size_t i=0;i<n;++i){out+=d[p[i]>>4];out+=d[p[i]&15];}return out;}
int main(int argc,char**){
 if(argc==1){unsigned char b[4096];size_t n=0;if(API(subtract_probe)(b,sizeof(b),&n)||!n)return 2;std::cout<<hex(b,n)<<'\n';if(API(subtract_probe)(b,1,&n)!=3||n!=0)return 3;return 0;}
 auto*L=luaL_newstate();if(!L)return 4;
 std::string source{std::istreambuf_iterator<char>(std::cin),{}};
 if(source.size()>65536||luaL_loadbufferx(L,source.data(),source.size(),"@trusted-numeric","t")||lua_pcall(L,0,LUA_MULTRET,0)||lua_gettop(L)!=1||!lua_isnumber(L,1))return 5;
 double v=lua_tonumber(L,1);uint64_t bits;std::memcpy(&bits,&v,8);if(argc>2)std::cout<<(lua_isinteger(L,1)?"integer:":"float:");std::cout<<std::hex<<std::setfill('0')<<std::setw(16)<<bits<<'\n';lua_close(L);
}
'''


def patched(text):
    start = text.index('static int finishbinexpneg')
    end = text.index('\n}', start) + 2
    body = text[start:end]
    needle = 'lua_Integer i2 = e2->u.ival;'
    if body.count(needle) != 1 or 'event == TM_SUB && i2 == 0' in body:
        raise RuntimeError('Unreviewed subtraction comparison patch context')
    updated = body.replace(needle, needle + '\n    if (event == TM_SUB && i2 == 0) return 0;')
    return text[:start] + updated + text[end:]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--release', action='append')
    parser.add_argument('--model-reference', action='store_true', help='Build the restricted core reference image against each exact Lua 5.4 library and check modeled returns')
    args = parser.parse_args()
    releases = {p['release']: p['archive_sha256'] for p in json.loads((ROOT / 'data/lua-profiles.json').read_text())['profiles']}
    releases.update(target.RELEASES)
    if args.release and set(args.release) - releases.keys():
        parser.error('Unknown exact Lua release')
    results = []
    for release, archive_sha in releases.items():
        if args.release and release not in args.release:
            continue
        with tempfile.TemporaryDirectory(prefix='hye-numeric-profile-') as directory:
            root = Path(directory)
            with urllib.request.urlopen(f'https://www.lua.org/ftp/lua-{release}.tar.gz', timeout=30) as reply:
                data = reply.read(1024 * 1024 + 1)
            if len(data) > 1024 * 1024 or hashlib.sha256(data).hexdigest() != archive_sha:
                raise RuntimeError('Numeric reference archive identity differs')
            target.unpack(data, root, release)
            source = root / ('lua-' + release) / 'src'
            file = source / 'lcode.c'
            original = file.read_text()
            for variant in ['original', 'zero_subtract_fix']:
                file.write_text(original if variant == 'original' else patched(original))
                subprocess.run(['make', '-C', str(source), 'clean'], check=True, stdout=subprocess.DEVNULL, timeout=30)
                subprocess.run(['make', '-C', str(source), '-j2', 'liblua.a', 'MYCFLAGS=-fPIC -DLUA_USE_LINUX'], check=True, stdout=subprocess.DEVNULL, timeout=90)
                bridge = ROOT / ('crates/editor-core/native/lua_syntax.cpp' if release.startswith('5.4.') else 'crates/editor-lua55/native/lua55.cpp')
                cpp, binary = root / 'fixture.cpp', root / 'fixture'
                cpp.write_text(CPP)
                subprocess.run(['c++', '-std=c++17', '-I' + str(source), str(bridge), str(cpp), str(source / 'liblua.a'), '-lm', '-ldl', '-o', str(binary)], check=True, timeout=60)
                probes = [subprocess.check_output([str(binary)], timeout=4, text=True, env={'LC_ALL': 'C'}).strip() for _ in range(3)]
                if len(set(probes)) != 1:
                    raise RuntimeError('Compiler probe differs across fresh states')
                observations = [subprocess.check_output([str(binary), 'trusted'], input=code, text=True, timeout=4, env={'LC_ALL': 'C'}).strip() for code in SOURCES]
                right = [subprocess.check_output([str(binary), 'trusted', 'kind'], input=PRELUDE + 'return (' + expression + ')', text=True, timeout=4, env={'LC_ALL': 'C'}).strip() for expression in EXPRESSIONS]
                if any(v not in ['integer:0000000000000000', 'float:0000000000000000', 'float:8000000000000000'] for v in right):
                    raise RuntimeError('Right-hand numeric reference is not zero')
                mode = 'rewrites_constant_integer_zero' if variant == 'original' else 'preserves_subtraction'
                if observations[0] != ('0000000000000000' if variant == 'original' else '8000000000000000') or observations[9] != '8000000000000000':
                    raise RuntimeError('Native numeric reference controls differ')
                results.append({'release': release, 'archive_sha256': archive_sha, 'variant': variant, 'lcode_sha256': hashlib.sha256(file.read_bytes()).hexdigest(), 'probe_sha256': hashlib.sha256(bytes.fromhex(probes[0])).hexdigest(), 'mode': mode, 'observations': observations, 'right_kinds': [v.split(':')[0] for v in right]})
                if args.model_reference and release.startswith('5.4.'):
                    pc = root / ('pc-' + variant)
                    pc.mkdir(exist_ok=True)
                    # A unique archive name prevents an earlier system -L path
                    # (e.g. RE2's) resolving generic -llua to another Lua family.
                    archive_name = 'hye_numeric_lua_' + release.replace('.', '_') + '_' + variant
                    shutil.copyfile(source / 'liblua.a', pc / ('lib' + archive_name + '.a'))
                    (pc / 'lua5.4.pc').write_text(f'Name: Lua\nDescription: private numeric model reference\nVersion: {release}\nCflags: -I{source}\nLibs: -L{pc} -l{archive_name} -lm -ldl\n')
                    env = dict(os.environ, PKG_CONFIG_PATH=str(pc) + (os.pathsep + os.environ['PKG_CONFIG_PATH'] if os.environ.get('PKG_CONFIG_PATH') else ''), CARGO_TARGET_DIR=str(root / 'model-target'))
                    env.pop('PKG_CONFIG_LIBDIR', None)
                    subprocess.run(['cargo', 'build', '--locked', '-p', 'hyprland-editor-core', '--example', 'numeric_reference'], cwd=ROOT, env=env, check=True, timeout=180)
                    model = root / 'model-target/debug/examples/numeric_reference'
                    dependencies = subprocess.check_output(['readelf', '-d', str(model)], text=True, timeout=10)
                    if any('NEEDED' in line and 'liblua' in line.lower() for line in dependencies.splitlines()):
                        raise RuntimeError('Private numeric model unexpectedly depends on a system Lua library')
                    for version in ['0.56.2', 'audited-main']:
                        for index, code in enumerate(SOURCES):
                            program = {'version': version, 'profile': 'synthetic_emit', 'root': 'root', 'documents': [{'id': 'root', 'source': code, 'sha256': hashlib.sha256(code.encode()).hexdigest(), 'load_name': list(b'/numeric.lua')}], 'bindings': [], 'initial_package_path': [], 'initial_package_path_unknown_tail': False, 'working_directory': None}
                            reply = subprocess.run([str(model)], input=json.dumps(program).encode(), capture_output=True, check=True, timeout=6, env={})
                            if len(reply.stdout) > 4 * 1024 * 1024:
                                raise RuntimeError('Numeric model output exceeds its bound')
                            report = json.loads(reply.stdout)
                            values = report['returns']
                            compiler = report['compiler_profile']
                            if report['halt'] is not None or len(values) != 1 or values[0]['type'] != 'float' or struct.pack('>d', values[0]['value']).hex() != observations[index] or compiler['release'] != release or f'Lua {release} ' not in compiler['library_identity'] or compiler['numeric_codegen']['probe_sha256'] != results[-1]['probe_sha256'] or report['runtime_verified']:
                                raise RuntimeError(f'Numeric model/native compiler mismatch: {release}/{variant}/{version}/{EXPRESSIONS[index]}: compiler={compiler}, returns={values}, halt={report["halt"]}')
                    print(f'{release}/{variant}: restricted model matches {2*len(SOURCES)} native cases across both audited snapshots', flush=True)
                print(f'{release}/{variant}: fixed compile/dump fingerprint and {len(observations)} trusted numeric cases verified; temporary source/builds removed on scope exit', flush=True)
    content = {'format': 'hye-lua-numeric-fixtures-1', 'scope': 'Official hash-pinned releases and a narrowly applied upstream subtraction-zero fix. Fixed compile/dump probe is never invoked. Only trusted reference expressions execute; no imported configuration, complete compiler, target compositor or dependency-security certificate.', 'fix_source': 'https://github.com/lua/lua/commit/b35736b9d71182bae67492af2a387b3f6e89fd05', 'expressions': EXPRESSIONS, 'sources': SOURCES, 'profiles': results}
    runtime = {'format': 'hye-lua-numeric-codegen-1', 'scope': 'Fixed stripped bytecode fingerprints for official releases and the explicitly applied zero-subtraction fix. No probe is invoked in analysis; no complete compiler, selected compositor or security certificate.', 'probe_source': 'return function(x) return x - 0 end', 'fix_source': content['fix_source'], 'profiles': [{k:v for k,v in row.items() if k not in ['observations','right_kinds']} for row in results]}
    if args.check:
        recorded_runtime = json.loads(OUTPUT.read_text())
        for row in runtime['profiles']:
            if row not in recorded_runtime['profiles']:
                raise RuntimeError('Compiler fingerprint/mode reference differs')
        if {k:v for k,v in runtime.items() if k != 'profiles'} != {k:v for k,v in recorded_runtime.items() if k != 'profiles'}:
            raise RuntimeError('Compiler fingerprint context differs')
        expected = json.loads(FIXTURES.read_text())
        for row in results:
            if row not in expected['profiles']:
                raise RuntimeError('Native numeric code-generation reference differs')
        if {k:v for k,v in content.items() if k != 'profiles'} != {k:v for k,v in expected.items() if k != 'profiles'}:
            raise RuntimeError('Native numeric reference corpus/context differs')
    else:
        if args.release:
            parser.error('Regeneration must retain every exact supported release and variant')
        OUTPUT.write_text(json.dumps(runtime, indent=2) + '\n')
        FIXTURES.write_text(json.dumps(content, indent=2) + '\n')


if __name__ == '__main__':
    main()
