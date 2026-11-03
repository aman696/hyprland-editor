#!/usr/bin/env python3
"""Trusted Lua 5.5 language-boundary fixtures; never an application evaluator.

Manual references: https://www.lua.org/manual/5.5/manual.html#8.1 and #3.3.7.
Only these literal cases execute with the base library, a bounded allocator,
instruction hook and child resource limits. No imported configuration executes.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import native_fixture_context as context
from monitor_source_probe import limits

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/lua55-language-fixtures.json'
CASES = [
    ('global_name_compatibility', 'local global=1; return global', True),
    ('declared_global', 'global x; x=3; return x', True),
    ('undeclared_name', 'global x; y=1', False),
    ('global_constant_write', 'global<const> x; x=1', False),
    ('global_constant_initialization', 'global<const> x=7; return x', True),
    ('global_initialization_does_not_replace', 'x=1; global x=2; return x', True),
    ('global_without_initializer_preserves', 'x=4; global x; return x', True),
    ('global_default_scope_restored', 'x=1; do global y; y=2 end; x=3; return x', True),
    ('global_constant_star', 'global<const> *; x=1', False),
    ('environment_write_bypasses_global_const', 'global<const> x; _ENV.x=9; return x', True),
    ('numeric_for_read_only', 'for i=1,2 do i=3 end', False),
    ('generic_for_control_read_only', 'for k,v in pairs({x=1}) do k=2 end', False),
    ('generic_for_secondary_writable', 'for k,v in pairs({x=1}) do v=2; return v end', True),
    ('shadow_loop_control', 'for i=1,2 do local i=i+3; return i end', True),
    ('nil_error_object', 'return pcall(error,nil)', True),
    ('false_error_object', 'return pcall(error,false)', True),
]
CPP = r'''
#include <lua.hpp>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <iterator>
#include <string>
struct Budget{size_t bytes=0;};
void* alloc(void* ud,void* p,size_t old,size_t n){auto*b=static_cast<Budget*>(ud);if(!p)old=0;if(old>b->bytes)return nullptr;if(!n){std::free(p);b->bytes-=old;return nullptr;}constexpr size_t cap=8*1024*1024;if(n>cap||b->bytes-old>cap-n)return nullptr;void*r=std::realloc(p,n);if(r)b->bytes=b->bytes-old+n;return r;}
std::string hex(const char*p,size_t n){const char*d="0123456789abcdef";std::string out;for(size_t i=0;i<n;++i){auto c=static_cast<unsigned char>(p[i]);out+=d[c>>4];out+=d[c&15];}return out;}
void hook(lua_State*L,lua_Debug*){luaL_error(L,"fixture instruction bound");}
int main(){
 Budget budget;auto*L=lua_newstate(alloc,&budget,0);if(!L)return 2;
 luaL_requiref(L,"_G",luaopen_base,1);lua_pop(L,1);lua_sethook(L,hook,LUA_MASKCOUNT,10000);
 std::string source{std::istreambuf_iterator<char>(std::cin),{}};
 if(source.size()>65536){lua_close(L);return 3;}
 int compiled=luaL_loadbufferx(L,source.data(),source.size(),"@language-fixture","t");
 std::cout<<"{\"compiled\":"<<(compiled==LUA_OK?"true":"false")<<",\"runtime_ok\":";
 int status=compiled;if(compiled==LUA_OK)status=lua_pcall(L,0,LUA_MULTRET,0);
 std::cout<<(compiled==LUA_OK?(status==LUA_OK?"true":"false"):"null")<<",\"values\":[";
 for(int i=1;i<=lua_gettop(L);++i){if(i>1)std::cout<<',';int type=lua_type(L,i);
  if(type==LUA_TNIL)std::cout<<"{\"type\":\"nil\"}";
  else if(type==LUA_TBOOLEAN)std::cout<<"{\"type\":\"bool\",\"value\":"<<(lua_toboolean(L,i)?"true":"false")<<'}';
  else if(lua_isinteger(L,i))std::cout<<"{\"type\":\"int\",\"value\":"<<lua_tointeger(L,i)<<'}';
  else if(type==LUA_TSTRING){size_t n;auto*p=lua_tolstring(L,i,&n);std::cout<<"{\"type\":\"bytes\",\"hex\":\""<<hex(p,n)<<"\"}";}
  else{lua_close(L);return 4;}
 }
 std::cout<<"]}\n";lua_close(L);if(budget.bytes)return 5;
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--output', type=Path)
    context.add_arguments(parser)
    args = parser.parse_args()
    if args.lua_package != 'lua5.5':
        raise RuntimeError('Lua 5.5-specific fixtures require the explicit lua5.5 package')
    flags = context.flags(args)
    rows = []
    with tempfile.TemporaryDirectory(prefix='hye-lua55-language-') as directory:
        root = Path(directory)
        cpp, binary = root / 'fixture.cpp', root / 'fixture'
        cpp.write_text(context.guard(args) + CPP)
        subprocess.run([args.compiler, '-std=' + args.cpp_standard, str(cpp), *flags, '-o', str(binary)], check=True, timeout=45)
        for name, code, compiled in CASES:
            result = subprocess.run([str(binary)], input=code, text=True, capture_output=True, check=True, timeout=4, preexec_fn=limits, env={'LC_ALL': 'C'})
            value = json.loads(result.stdout)
            if value['compiled'] != compiled:
                raise RuntimeError(f'Unexpected compile outcome: {name}: {value}')
            rows.append({'case': name, 'source': code, 'observation': value})
    content = {'format': 'hye-lua55-language-fixtures-1', 'scope': 'Trusted Lua 5.5 default official source compatibility flags: contextual global declarations, loop-control and error-object cases with base library, explicit allocator seed and bounds. Reserved-global builds are not established. No imported source, grammar/evaluator, complete language or compositor compatibility certificate.', 'cases': rows}
    rendered = json.dumps(content, indent=2) + '\n'
    if args.check and OUTPUT.read_text() != rendered:
        raise RuntimeError('Lua 5.5 language-boundary observations differ')
    if args.output:
        args.output.write_text(rendered)
    elif not args.check:
        OUTPUT.write_text(rendered)
    print(f'{len(rows)} Lua 5.5 language-boundary observations verified; temporary builds removed')


if __name__ == '__main__':
    main()
