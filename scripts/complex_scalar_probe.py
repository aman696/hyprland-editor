#!/usr/bin/env python3
"""Observe pinned complex Lua parsers on trusted library-free fixtures.

Only original parser bodies and primary storage/conversion operations execute.
Graphics caches are inert. These are parser observations, not compositor tests.
All downloaded source buffers and temporary builds are discarded on exit.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import resource
import shlex
import subprocess
import sys
import tempfile
import urllib.request

sys.dont_write_bytecode = True
from scalar_registration_probe import block

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'data/complex-scalar-fixtures.json'
KINDS = ['Color', 'Gradient', 'Vec2', 'CssGap', 'FontWeight']
BASE = ['nil', 'false', 'true', '0', '1', '-1', '1.0', '3.5', '-0.5', "''", "'1'", "'3.5'", "' 0x1p0 '", '{}', 'function()end']
COLORS = [repr(s) for s in ['0x', '0xFF123456', '0x123456789ABCDEF0', '0XFF123456', '#123', '#123456', '#12345678', '#ff', '#xyz', 'rgba(FF000080)', 'rgba(1, 2, 3, 0.5)', 'rgba(1,2,3,2)', 'rgba(256,2,3,1)', 'rgba(+1,2,3,1)', 'rgb(0x00ff)', 'rgb(123456)', 'rgb(1,2,3)', 'rgb( 1 , 2 , 3 )', 'rgb(1,2)', '4294967295', '4294967296', '0', '1.0', ' 0xFF123456 ', '0xFFFFFFFFFFFFFFFF']]
CASES = {
    'Color': BASE + COLORS + ["'0xFF123456\\0suffix'", '4294967295', '4294967295.0'],
    'Gradient': BASE + COLORS + ["{colors={}}", "{colors={'#123'}}", "{colors={'#123','#456'},angle=45}", "{colors={1,2},angle='90'}", "{colors={'#123'},angle='bad'}", "{colors={'#123'},angle=false}", "{colors={false}}", "{colors={'bad'}}", "{colors={'#123'},angle=1/0}", "{colors={'#123'},angle=0/0}", "{colors={'#123'},angle=-0.0}", "{colors={'#123'},angle=0.1}", "{colors={'#123'},extra=123}", "{colors={['1']='#123'}}", "{colors={[1]='#123',[3]='#456'}}"],
    'Vec2': BASE + ["{1,2}", "{'1','2.5'}", "{true,2}", "{1,false}", "{x=1,y=2}", "{1,2,3}", "{[1]=1,[3]=3}", "{1,2,x=9,y=9}", "{-0.0,0.1}", "{16777217,16777217.0}", "{1/0,0/0}", "'1 2'", "'1,2'", "'1,,2'", "'1 2 tail'", "' 1 2 '", "'0x1p0 2'", "'nan 2'", "'1e300 2'", "'1 2\\0ignored'", "{0,10}", "{-1,2}", "{11,2}"],
    'CssGap': BASE + ["{top=3.5}", "{top='3.5'}", "{top=3.0}", "{top='3'}", "{top=true}", "{left=6,top=4}", "{top=0/0}", "{top=1/0}", "{[1]=9}", "{top=-1}", "{top=11}", "{top=9223372036854775807}", "3.9", "-0.9", "9007199254740993", "'3.9'", "{['top\\0suffix']=8}", "{top=11,right=false}"],
    'FontWeight': BASE + ["'bold'", "'BOLD'", "'ultraheavy'", "'normal'", "'normal '", "'700'", "'700.9'", "'bold\\0suffix'", '0', '1001', '65536', '9007199254740993', '-0.9'],
}
# Cover every normalized channel byte and every original weight name.
CASES['Color'] += [repr('0x'+f'{v:02X}'*4) for v in range(256)]
CASES['FontWeight'] += [repr(name) for name in ['thin','ultralight','light','semilight','book','normal','medium','semibold','bold','ultrabold','heavy','ultraheavy']]
HEADER = r'''
#include <lua.hpp>
#include <algorithm>
#include <bit>
#include <charconv>
#include <cmath>
#include <cstdint>
#include <expected>
#include <format>
#include <functional>
#include <iostream>
#include <limits>
#include <map>
#include <numbers>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>
#define sc static_cast
// Byte-equivalent C++26 string/view concatenation for C++23 fixture hosts.
std::string operator+(std::string left,std::string_view right){left.append(right);return left;}
namespace Config { using INTEGER=int64_t;struct VEC2{float x=0,y=0;}; }
enum Error { PARSE_ERROR_OK,PARSE_ERROR_OUT_OF_RANGE,PARSE_ERROR_BAD_VALUE,PARSE_ERROR_BAD_TYPE };
struct SParseError { Error errorCode=PARSE_ERROR_OK;std::string message; };
namespace Hyprgraphics { struct CColor {struct SSRGB{double r,g,b;};CColor(SSRGB){}int asOkLab(){return 0;} }; }
struct CHyprColor {double r=0,g=0,b=0,a=0;int m_okLab=0;CHyprColor(uint64_t);uint32_t getAsHex()const;};
struct CCssGapData {int64_t m_top=0,m_right=0,m_bottom=0,m_left=0;CCssGapData()=default;CCssGapData(int64_t a,int64_t b,int64_t c,int64_t d):m_top(a),m_right(b),m_bottom(c),m_left(d){}void reset(int64_t v){m_top=m_right=m_bottom=m_left=v;}};
struct CGradientValueData {std::vector<CHyprColor>m_colors;float m_angle=0;void updateColorsOk(){}void reset(CHyprColor c){m_colors={c};m_angle=0;}};
struct CFontWeightConfigValueData {int64_t m_value=400;
WEIGHT_MAP
};
struct CLuaConfigColor {int64_t m_data=0;bool m_bSetByUser=false;SParseError parse(lua_State*);};
struct CLuaConfigGradient {CGradientValueData m_data;bool m_bSetByUser=false;SParseError parse(lua_State*);};
struct CLuaConfigVec2 {Config::VEC2 m_data;std::function<std::expected<void,std::string>(const Config::VEC2&)>m_validator;bool m_bSetByUser=false;SParseError parse(lua_State*);};
struct CLuaConfigCssGap {CCssGapData m_data;std::optional<int64_t>m_min,m_max;bool m_bSetByUser=false;SParseError parse(lua_State*);};
struct CLuaConfigFontWeight {CFontWeightConfigValueData m_data;bool m_bSetByUser=false;SParseError parse(lua_State*);};
namespace Hyprutils::String {
std::string_view trim(const std::string_view&);
bool isNumber2(const std::string_view&,bool allowfloat=false);
bool isNumber(const std::string&,bool allowfloat=false);
}
namespace Config::ParserUtils {std::expected<int64_t,std::string>parseColor(std::string_view);}
using namespace Config;using namespace Hyprutils::String;
'''
MAIN = r'''
void observed(lua_State*L,int index,int depth=0){
 if(depth>8)std::abort();index=lua_absindex(L,index);const auto type=lua_type(L,index);
 switch(type){
 case LUA_TNIL:std::cout<<R"({"type":"nil"})";break;
 case LUA_TBOOLEAN:std::cout<<R"({"type":"bool","value":)"<<(lua_toboolean(L,index)?"true":"false")<<'}';break;
 case LUA_TNUMBER:if(lua_isinteger(L,index))std::cout<<R"({"type":"int","value":)"<<lua_tointeger(L,index)<<'}';else std::cout<<R"({"type":"float","value":{"bits":")"<<std::format("{:016x}",std::bit_cast<uint64_t>(lua_tonumber(L,index)))<<R"("}})";break;
 case LUA_TSTRING:{size_t count=0;const auto*b=lua_tolstring(L,index,&count);std::cout<<R"({"type":"bytes","value":[)";for(size_t i=0;i<count;++i){if(i)std::cout<<',';std::cout<<unsigned(static_cast<unsigned char>(b[i]));}std::cout<<"]}";break;}
 case LUA_TTABLE:{std::cout<<R"({"type":"table","value":[)";bool first=true;lua_pushnil(L);while(lua_next(L,index)){if(!first)std::cout<<',';first=false;std::cout<<'[';observed(L,-2,depth+1);std::cout<<',';observed(L,-1,depth+1);std::cout<<']';lua_pop(L,1);}std::cout<<"]}";break;}
 default:std::cout<<R"({"type":"function","value":"fixture"})";break;
 }
}
void emit(uint32_t v){std::cout<<v;}
void emit(int64_t v){std::cout<<v;}
void emit(const Config::VEC2&v){std::cout<<'['<<std::bit_cast<uint32_t>(v.x)<<','<<std::bit_cast<uint32_t>(v.y)<<']';}
void emit(const CCssGapData&v){std::cout<<'['<<v.m_top<<','<<v.m_right<<','<<v.m_bottom<<','<<v.m_left<<']';}
void emit(const CFontWeightConfigValueData&v){std::cout<<v.m_value;}
void emit(const CGradientValueData&v){std::cout<<R"({"colors":[)";for(size_t i=0;i<v.m_colors.size();++i){if(i)std::cout<<',';std::cout<<v.m_colors[i].getAsHex();}std::cout<<R"(],"angle_bits":)"<<std::bit_cast<uint32_t>(v.m_angle)<<'}';}
template<class T>void run(lua_State*L,T&value){
 const auto top=lua_gettop(L);std::cout<<R"({"input":)";observed(L,-1);std::cout<<R"(,"root_raw_length":)"<<(lua_istable(L,-1)?lua_rawlen(L,-1):0);
 const auto result=value.parse(L);if(lua_gettop(L)!=top)std::abort();
 std::cout<<R"(,"error_code":)"<<result.errorCode<<R"(,"set_by_user":)"<<(value.m_bSetByUser?"true":"false");
 std::cout<<R"(,"stored":)";emit(value.m_data);std::cout<<"}\n";
}
int main(int argc,char**argv){
 if(argc!=3)return 2;const std::string code="return "+std::string((std::istreambuf_iterator<char>(std::cin)),{});
 auto*L=luaL_newstate();if(!L)return 3;if(luaL_loadbufferx(L,code.data(),code.size(),"@trusted-complex-fixture","t")||lua_pcall(L,0,1,0))return 4;
 const std::string kind=argv[1],profile=argv[2];
 if(kind=="Color"){CLuaConfigColor v;run(L,v);}
 else if(kind=="Gradient"){CLuaConfigGradient v;run(L,v);}
 else if(kind=="Vec2"){CLuaConfigVec2 v;if(profile=="bounded")v.m_validator=[](const Config::VEC2&v)->std::expected<void,std::string>{if(v.x<0||v.y<0||v.x>10||v.y>10)return std::unexpected("fixture bounds");return {};};run(L,v);}
 else if(kind=="CssGap"){CLuaConfigCssGap v;if(profile=="bounded"){v.m_min=1;v.m_max=10;}run(L,v);}
 else if(kind=="FontWeight"){CLuaConfigFontWeight v;run(L,v);}
 else return 5;lua_close(L);
}
'''


def build_source(sources, version):
    get = lambda path: sources[version, path]
    data = get('src/config/shared/complex/ComplexDataTypes.hpp')
    start = data.index('inline static const auto WEIGHTS')
    weights = data[start:data.index('};', start) + 2]
    code = HEADER.replace('WEIGHT_MAP', weights)
    code += get('hyprutils/include/hyprutils/string/Numeric.hpp')
    string = get('hyprutils/src/string/String.cpp')
    for signature in ['std::string_view Hyprutils::String::trim(', 'bool Hyprutils::String::isNumber2(', 'bool Hyprutils::String::isNumber(']:
        code += block(string, signature)
    color = get('src/helpers/Color.cpp')
    code += '\n'.join(line for line in color.splitlines() if line.startswith('#define ')) + '\n'
    code += block(color, 'CHyprColor::CHyprColor(uint64_t hex)')
    code += block(color, 'uint32_t CHyprColor::getAsHex() const')
    parsing = get('src/config/shared/parserUtils/ParserUtils.cpp')
    code += block(parsing, 'static std::expected<uint64_t, std::string> parseHex(')
    code += block(parsing, 'std::expected<int64_t, std::string> ParserUtils::parseColor(')
    code += block(get('src/config/lua/types/LuaConfigColor.cpp'), 'static std::expected<CHyprColor, std::string> parseColorString(')
    code += block(get('src/config/lua/types/LuaConfigCssGap.cpp'), 'static std::string checkRange(')
    for kind in KINDS:
        code += block(get(f'src/config/lua/types/LuaConfig{kind}.cpp'), f'SParseError CLuaConfig{kind}::parse(')
    return code + MAIN


def fetch(row):
    with urllib.request.urlopen(row['source'], timeout=30) as reply:
        data = reply.read()
    if hashlib.sha256(data).hexdigest() != row['sha256']:
        raise RuntimeError(f"Pinned complex reference differs: {row['path']}")
    return (row['version'], row['path']), data.decode()


def limits():
    for target, value in [(resource.RLIMIT_AS, 256 * 1024 * 1024), (resource.RLIMIT_CPU, 3), (resource.RLIMIT_STACK, 1024 * 1024), (resource.RLIMIT_CORE, 0)]:
        resource.setrlimit(target, (value, value))


def normalize(value):
    if isinstance(value, dict):
        value = {key: normalize(item) for key, item in value.items()}
        if value.get('type') == 'table':
            value['value'].sort(key=lambda pair: json.dumps(pair[0], sort_keys=True))
    elif isinstance(value, list):
        value = [normalize(item) for item in value]
    return value


def observe(binary, version):
    observations = []
    for kind in KINDS:
        for profile in (['unbounded', 'bounded'] if kind in ['Vec2', 'CssGap'] else ['unbounded']):
            for expression in dict.fromkeys(CASES[kind]):
                result = subprocess.run([str(binary), kind, profile], input=expression, text=True, capture_output=True, check=True, timeout=5, preexec_fn=limits)
                observations.append(dict(version=version, kind=kind, profile=profile, expression=expression, **normalize(json.loads(result.stdout))))
    return observations


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    manifest = (ROOT / 'data/complex-scalar-sources.json').read_bytes()
    rows = json.loads(manifest)['files']
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        sources = dict(pool.map(fetch, rows))
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
    observations = []
    with tempfile.TemporaryDirectory(prefix='hye-complex-scalars-') as folder:
        root = Path(folder)
        for version in ['baseline', 'current']:
            cpp = root / f'{version}.cpp'
            cpp.write_text(build_source(sources, version))
            binary = root / version
            subprocess.run(['c++', '-std=c++23', str(cpp), *flags, '-o', str(binary)], check=True, timeout=90)
            cases = observe(binary, version)
            observations += cases
            print(f'{version}: {len(cases)} complex parser observations', flush=True)
    value = {'format': 'hye-complex-scalar-fixtures-1', 'sources_sha256': hashlib.sha256(manifest).hexdigest(),
             'scope': 'Pinned original five Lua parsers, color parsing/numeric helpers and primary color conversion; inert graphics/cache adapters and explicit fixture bounds. No compositor/native setter effects, arbitrary table layout, undefined numeric casts, locale/stdlib portability or edit ownership certificate.',
             'cases': observations}
    text = json.dumps(value, indent=2) + '\n'
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != text:
            raise SystemExit('Complex scalar observations differ')
    else:
        OUTPUT.write_text(text)


if __name__ == '__main__':
    main()
