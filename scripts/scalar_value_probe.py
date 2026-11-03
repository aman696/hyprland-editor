#!/usr/bin/env python3
"""Run pinned primitive parse methods on trusted private Lua fixture values.

Registry bounds/maps are explicit fixture inputs, not all setting registrations.
No user configuration executes. Original source/build files are temporary.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import resource
import subprocess
import tempfile
import urllib.request
import native_fixture_context as context

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/scalar-value-fixtures.json'
TYPES_SHA256 = '337556a3fbedda3e34f31f600f4a44d4e85f53a04220f3c90d8394f98d1eb638'
KINDS = ['Bool', 'Int', 'Float', 'String']
CASES = [
    'nil', 'false', 'true', '0', '1', '2', '-1', '3', '10', '11',
    '0.0', '1.0', '3.0', '3.5', '-0.0', '1/0', '-1/0', '0/0',
    "''", "'0'", "'1'", "'3'", "'3.0'", "' 0x1p0 '",
    "'enabled'", "'disabled'", "'beyond'", "'unknown'",
    "'enabled\\0suffix'", "'1\\0suffix'", "'hello\\0hidden'",
    '9223372036854775807', '-9223372036854775807-1', '16777217',
    '16777217.0', '{}', 'function()end',
]
HEADER = r'''
#include <lua.hpp>
#include <bit>
#include <cstdint>
#include <expected>
#include <format>
#include <functional>
#include <iostream>
#include <optional>
#include <string>
#include <unordered_map>
namespace Config { using BOOL=bool; using INTEGER=int64_t; using FLOAT=float; using STRING=std::string; }
enum Error {PARSE_ERROR_OK, PARSE_ERROR_OUT_OF_RANGE, PARSE_ERROR_BAD_VALUE, PARSE_ERROR_BAD_TYPE};
struct SParseError {Error errorCode=PARSE_ERROR_OK;std::string message;};
struct CLuaConfigBool {bool m_data=true,m_bSetByUser=false;SParseError parse(lua_State*);};
struct CLuaConfigInt {
 int64_t m_data=17;bool m_bSetByUser=false;
 std::optional<int64_t> m_min,m_max;
 std::optional<std::unordered_map<std::string,int64_t>> m_map;
 SParseError parse(lua_State*);
};
struct CLuaConfigFloat {
 float m_data=17.25;bool m_bSetByUser=false;
 std::optional<float> m_min,m_max;
 SParseError parse(lua_State*);
};
struct CLuaConfigString {
 std::string m_data="initial";bool m_bSetByUser=false;
 std::function<std::expected<void,std::string>(const std::string&)> m_validator;
 SParseError parse(lua_State*);
};
std::string hex(const std::string& s){std::string out;for(unsigned char c:s)out+=std::format("{:02x}",c);return out;}
void emit(bool v){std::cout<<(v?"true":"false");}
void emit(int64_t v){std::cout<<v;}
void emit(float v){std::cout<<'"'<<std::format("{:08x}",std::bit_cast<uint32_t>(v))<<'"';}
void emit(const std::string& v){std::cout<<'"'<<hex(v)<<'"';}
void input(lua_State* L){
 const auto type=lua_type(L,-1);
 std::cout<<"{\"kind\":\""<<lua_typename(L,type)<<'"';
 if(type==LUA_TNUMBER){
  if(lua_isinteger(L,-1))std::cout<<",\"integer\":\""<<lua_tointeger(L,-1)<<'"';
  else std::cout<<",\"float_bits\":\""<<std::format("{:016x}",std::bit_cast<uint64_t>(lua_tonumber(L,-1)))<<'"';
 }else if(type==LUA_TBOOLEAN)std::cout<<",\"boolean\":"<<(lua_toboolean(L,-1)?"true":"false");
 else if(type==LUA_TSTRING){size_t count=0;const char* bytes=lua_tolstring(L,-1,&count);std::cout<<",\"bytes_hex\":\""<<hex(std::string(bytes,count))<<'"';}
 std::cout<<'}';
}
template<class T>void run(lua_State* L,T& v){
 std::cout<<"{\"input\":";input(L);
 const auto initial=v.m_data;const auto before=lua_gettop(L);auto result=v.parse(L);
 if(lua_gettop(L)!=before)std::abort();
 std::cout<<",\"error_code\":"<<result.errorCode<<",\"has_message\":"<<(result.message.empty()?"false":"true")<<",\"set_by_user\":"<<(v.m_bSetByUser?"true":"false")<<",\"initial\":";
 emit(initial);std::cout<<",\"stored\":";emit(v.m_data);
 std::cout<<",\"stack_type_after\":\""<<lua_typename(L,lua_type(L,-1))<<"\"}\n";
}
'''
MAIN = r'''
int main(int argc,char**argv){
 if(argc!=3)return 2;std::string input((std::istreambuf_iterator<char>(std::cin)),{});
 lua_State* L=luaL_newstate();if(!L)return 3;
 const auto code="return "+input;
 if(luaL_loadbufferx(L,code.data(),code.size(),"@trusted-fixture","t")||lua_pcall(L,0,1,0))return 4;
 std::string kind=argv[1],profile=argv[2];
 if(kind=="Bool"){CLuaConfigBool v;run(L,v);}
 else if(kind=="Int"){CLuaConfigInt v;if(profile!="unbounded"){v.m_min=0;v.m_max=10;}if(profile=="mapped")v.m_map=std::unordered_map<std::string,int64_t>{{"enabled",1},{"disabled",0},{"beyond",11},{"3.0",7}};run(L,v);}
 else if(kind=="Float"){CLuaConfigFloat v;if(profile!="unbounded"){v.m_min=0;v.m_max=profile=="precision"?16777216:10;}run(L,v);}
 else if(kind=="String"){CLuaConfigString v;run(L,v);}
 else return 5;
 lua_close(L);
}
'''


def function(source, signature):
    start = source.index(signature)
    brace = source.index('{', start)
    depth, end = 1, brace + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end] + '\n'


def fetch(row):
    with urllib.request.urlopen(row['source'], timeout=30) as reply:
        data = reply.read()
    if hashlib.sha256(data).hexdigest() != row['sha256']:
        raise RuntimeError('Pinned scalar parser/type source differs')
    return (row['version'], row['path']), data.decode()


def child_limits():
    resource.setrlimit(resource.RLIMIT_AS, (256*1024*1024,)*2)
    resource.setrlimit(resource.RLIMIT_CPU, (2,)*2)
    resource.setrlimit(resource.RLIMIT_STACK, (1024*1024,)*2)
    resource.setrlimit(resource.RLIMIT_CORE, (0,)*2)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--output', type=Path)
    context.add_arguments(parser)
    args = parser.parse_args()
    context.require_output(args, OUTPUT)
    contract = json.loads((ROOT/'docs/hyprland-implementation-contracts.json').read_text())
    paths = [f'src/config/lua/types/LuaConfig{k}.cpp' for k in KINDS]
    rows = [r for r in contract['source_evidence'] if r['path'] in paths]
    for version, commit in [('baseline', contract['baseline_commit']), ('current', contract['current_commit'])]:
        rows.append({'version': version, 'path': 'src/config/shared/Types.hpp',
                     'source': f'https://raw.githubusercontent.com/hyprwm/Hyprland/{commit}/src/config/shared/Types.hpp', 'sha256': TYPES_SHA256})
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        sources = dict(pool.map(fetch, rows))
    for version in ['baseline', 'current']:
        types = sources[(version, 'src/config/shared/Types.hpp')]
        if not all(alias in types for alias in ['typedef bool', 'typedef int64_t', 'typedef float', 'typedef std::string']):
            raise RuntimeError('Review fixture storage types against upstream aliases')
    flags = context.flags(args)
    observations = []
    with tempfile.TemporaryDirectory(prefix='hye-scalar-values-') as folder:
        root = Path(folder)
        for version in ['baseline', 'current']:
            cpp = root/f'{version}.cpp'
            code = context.guard(args) + HEADER
            for kind in KINDS:
                code += function(sources[(version, f'src/config/lua/types/LuaConfig{kind}.cpp')], f'SParseError CLuaConfig{kind}::parse(lua_State* s)')
            cpp.write_text(code+MAIN)
            binary = root/version
            subprocess.run([args.compiler, '-std='+args.cpp_standard, str(cpp), *flags, '-o', str(binary)], check=True, timeout=90)
            count = 0
            for kind in KINDS:
                profiles = ['unbounded', 'bounded', 'mapped'] if kind == 'Int' else ['unbounded', 'bounded', 'precision'] if kind == 'Float' else ['unbounded']
                for profile in profiles:
                    for expression in CASES:
                        result = subprocess.run([str(binary), kind, profile], input=expression, text=True, capture_output=True, check=True, timeout=5, preexec_fn=child_limits)
                        observations.append({'version': version, 'kind': kind, 'profile': profile, 'expression': expression, **json.loads(result.stdout)})
                        count += 1
            print(f'{version}: {count} exact primitive scalar parse observations', flush=True)
    result = {'format': 'hye-scalar-value-fixtures-1', 'source_files': rows,
              'evidence_scope': 'Unmodified pinned Bool/Int/Float/String parse methods; explicit inert storage and fixture bounds/maps, no string validator. Stored FLOAT is native binary32 bits. No all-setting registration, formatted diagnostic, effect or runtime-state certificate.', 'cases': observations}
    encoded = json.dumps(result, indent=2)+'\n'
    output = args.output or OUTPUT
    if args.check:
        if not output.exists() or output.read_text() != encoded:
            raise SystemExit('Primitive scalar observations differ')
    else:
        output.write_text(encoded)


if __name__ == '__main__':
    main()
