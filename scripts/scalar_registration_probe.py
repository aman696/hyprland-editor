#!/usr/bin/env python3
"""Observe pinned setting factories/options with inert services and complex defaults.

The four primitive value classes, factory, option structs, registration function
and validators are original source. No compositor services or Lua execute.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import resource
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT/'data/scalar-registration-fixtures.json'
KINDS = ['Bool','Int','Float','String','Color','CssGap','Vec2','FontWeight','Gradient']
PRIMITIVES = KINDS[:4]
HEADER = r'''
#include <algorithm>
#include <bit>
#include <cstdint>
#include <cstdlib>
#include <expected>
#include <format>
#include <functional>
#include <iostream>
#include <limits>
#include <map>
#include <memory>
#include <optional>
#include <ranges>
#include <set>
#include <string>
#include <type_traits>
#include <tuple>
#include <typeinfo>
#include <unordered_map>
#include <utility>
#include <vector>
template<class T>using UP=std::unique_ptr<T>;
template<class T,class...A>UP<T>makeUnique(A&&...a){return std::make_unique<T>(std::forward<A>(a)...);}
template<class T,class U>T dc(U* value){return dynamic_cast<T>(value);}
template<class T>using SP=std::shared_ptr<T>;
using StringValidator=std::function<std::expected<void,std::string>(const std::string&)>;
auto& captured_validators(){static std::map<const void*,StringValidator> values;return values;}
template<class T,class...A>SP<T>makeShared(A&&...a){
 StringValidator validator;
 if constexpr(sizeof...(A)>=4){auto tuple=std::forward_as_tuple(a...);if constexpr(requires{std::get<3>(tuple).validator;}){if constexpr(std::is_same_v<std::remove_cvref_t<decltype(std::get<3>(tuple).validator)>,StringValidator>)validator=std::get<3>(tuple).validator;}}
 auto value=std::make_shared<T>(std::forward<A>(a)...);
 if(validator)captured_validators()[value.get()]=std::move(validator);
 return value;
}
namespace Config {using BOOL=bool;using INTEGER=int64_t;using FLOAT=float;using STRING=std::string;struct VEC2{float x=0,y=0;};}
// No real config lookup, refresh, graphics or complex default constructor runs.
template<class T>class CConfigValue{public:CConfigValue()=default;CConfigValue(const char*){}bool good()const{return false;}const T&operator*()const{static T value{};return value;}};
struct CHyprColor{uint64_t value;CHyprColor(uint64_t v=0):value(v){}};
struct GapData{int64_t m_top;GapData(int64_t n=0):m_top(n){}};
struct WeightData{int64_t m_value;WeightData(int64_t n=400):m_value(n){}};
struct GradientData{GradientData(CHyprColor){}};

namespace Supplementary{using PropRefreshBits=uint64_t;
'''
CAPTURE = r'''
namespace Config::Values {
// Complex constructor defaults are intentionally not represented as native data.
template<class T,class O,int Tag>class InertComplex:public IValue{
 public:O options;T initial{};std::optional<int64_t> m_min,m_max;
 struct Gap{int64_t m_top;};struct Weight{int64_t m_value;};struct Gradient{std::vector<CHyprColor>m_colors;};
 InertComplex(const char*n,const char*d,T def={},O props={}):IValue(BASE_PROPS(props)),options(std::move(props)),initial(def){m_name=n;m_description=d;if constexpr(requires{options.min;}){m_min=options.min;m_max=options.max;}}
 auto defaultVal()const{if constexpr(Tag==1)return Gap{initial};else if constexpr(Tag==3)return Weight{initial};else if constexpr(Tag==4)return Gradient{{initial}};else return initial;}
 auto validator()const requires(Tag==2){return options.validator;}
 const std::type_info*underlying()const override{return &typeid(T);}
 void commence()override{}
};
using CColorValue=InertComplex<Config::INTEGER,SColorValueOptions,0>;
using CCssGapValue=InertComplex<Config::INTEGER,SCssGapValueOptions,1>;
using CVec2Value=InertComplex<Config::VEC2,SVec2ValueOptions,2>;
using CFontWeightValue=InertComplex<Config::INTEGER,SFontWeightValueOptions,3>;
using CGradientValue=InertComplex<CHyprColor,SGradientValueOptions,4>;
}
'''
MAIN = r'''
std::string hex(std::string_view bytes){std::string out;for(unsigned char c:bytes)out+=std::format("{:02x}",c);return out;}
void quoted(std::string_view bytes){std::cout<<'"'<<hex(bytes)<<'"';}
void boolean(bool v){std::cout<<(v?"true":"false");}
template<class T>void notice(T* v){if constexpr(requires{v->deprecationNotice();}){if(auto d=v->deprecationNotice())quoted(*d);else std::cout<<"null";}else std::cout<<"null";}
template<class T>StringValidator validator(T* v){if constexpr(requires{v->validator();})return v->validator();else {auto found=captured_validators().find(v);return found==captured_validators().end()?StringValidator{}:found->second;}}
template<class T>void bound(std::optional<T> v){if(!v){std::cout<<"null";return;}if constexpr(std::is_same_v<T,float>)std::cout<<'"'<<std::format("{:08x}",std::bit_cast<uint32_t>(*v))<<'"';else std::cout<<*v;}
template<class T>void limits(T*x){std::cout<<",\"minimum\":";bound(x->m_min);std::cout<<",\"maximum\":";bound(x->m_max);}
void names(const std::optional<std::unordered_map<std::string,int64_t>>& values){if(!values){std::cout<<"null";return;}std::map<std::string,int64_t> sorted(values->begin(),values->end());bool first=true;std::cout<<'[';for(const auto&[name,value]:sorted){if(!first)std::cout<<',';first=false;std::cout<<'[';quoted(name);std::cout<<','<<value<<']';}std::cout<<']';}
void validator_cases(const StringValidator& fn){std::cout<<"[";bool first=true;for(const auto&input:STRING_CASES){if(!first)std::cout<<',';first=false;std::cout<<'[';quoted(input);std::cout<<',';boolean(fn(input).has_value());std::cout<<']';}std::cout<<']';}
void vector_cases(const std::function<std::expected<void,std::string>(const Config::VEC2&)>&fn){
 const float nan=std::numeric_limits<float>::quiet_NaN(),inf=std::numeric_limits<float>::infinity();
 const std::vector<Config::VEC2> cases={{-inf,0},{nan,0},{0,nan},{0,0},{-250,-250},{250,250},{251,0},{-20000,-20000},{20000,20000},{20001,0},{-100,-100},{4000,4000},{4001,0},{500,500},{501,0},{1000,1000},{1001,0}};
 std::cout<<'[';bool first=true;for(const auto&v:cases){if(!first)std::cout<<',';first=false;std::cout<<'['<<std::bit_cast<uint32_t>(v.x)<<','<<std::bit_cast<uint32_t>(v.y)<<',';boolean(fn(v).has_value());std::cout<<']';}std::cout<<']';
}
void lua_options(const Config::Lua::ILuaConfigValue* value){
 using namespace Config::Lua;
 std::cout<<",\"lua\":{";
 if(auto*x=dynamic_cast<const CLuaConfigInt*>(value)){std::cout<<"\"kind\":\"Int\",\"native_default\":"<<x->m_default;limits(x);std::cout<<",\"names\":";names(x->m_map);}
 else if(auto*x=dynamic_cast<const CLuaConfigFloat*>(value)){std::cout<<"\"kind\":\"Float\",\"native_default\":\""<<std::format("{:08x}",std::bit_cast<uint32_t>(x->m_default))<<'"';limits(x);}
 else if(auto*x=dynamic_cast<const CLuaConfigBool*>(value)){std::cout<<"\"kind\":\"Bool\",\"native_default\":";boolean(x->m_default);}
 else if(auto*x=dynamic_cast<const CLuaConfigString*>(value)){std::cout<<"\"kind\":\"String\",\"native_default\":";quoted(x->m_default);std::cout<<",\"has_validator\":";boolean(bool(x->m_validator));if(x->m_validator){std::cout<<",\"validator_cases\":";validator_cases(x->m_validator);}}
 else if(auto*x=dynamic_cast<const CLuaConfigCssGap*>(value)){std::cout<<"\"kind\":\"CssGap\",\"native_default\":null";limits(x);}
 else if(auto*x=dynamic_cast<const CLuaConfigVec2*>(value)){std::cout<<"\"kind\":\"Vec2\",\"native_default\":null,\"has_validator\":";boolean(bool(x->m_validator));if(x->m_validator){std::cout<<",\"validator_cases\":";vector_cases(x->m_validator);}}
 else if(dynamic_cast<const CLuaConfigColor*>(value))std::cout<<"\"kind\":\"Color\",\"native_default\":null";
 else if(dynamic_cast<const CLuaConfigGradient*>(value))std::cout<<"\"kind\":\"Gradient\",\"native_default\":null";
 else if(dynamic_cast<const CLuaConfigFontWeight*>(value))std::cout<<"\"kind\":\"FontWeight\",\"native_default\":null";
 else std::abort();
 std::cout<<'}';
}
int main(){
 std::set<std::string> ids;bool first=true;std::cout<<"{\"values\":[";
 for(const auto&v:Config::Values::CONFIG_VALUES){
  if(!ids.insert(v->name()).second)return 2;
  if(!first)std::cout<<',';first=false;
  std::cout<<"{\"id_hex\":";quoted(v->name());std::cout<<",\"lua_key_hex\":";quoted(Config::Lua::CConfigManager::luaConfigValueName(v->name()));std::cout<<",\"description_hex\":";quoted(v->description());
  std::cout<<",\"deprecation_hex\":";notice(v.get());
  using namespace Config::Values;
  if(auto*x=dynamic_cast<CBoolValue*>(v.get())){std::cout<<",\"kind\":\"Bool\",\"native_default\":";boolean(x->defaultVal());}
  else if(auto*x=dynamic_cast<CIntValue*>(v.get())){
   std::cout<<",\"kind\":\"Int\",\"native_default\":"<<x->defaultVal();limits(x);
   std::cout<<",\"names\":";
   if(!x->m_map)std::cout<<"null";else{std::map<std::string,int64_t> sorted(x->m_map->begin(),x->m_map->end());bool first=true;std::cout<<'[';for(const auto&[name,value]:sorted){if(!first)std::cout<<',';first=false;std::cout<<'[';quoted(name);std::cout<<','<<value<<']';}std::cout<<']';}
  }
  else if(auto*x=dynamic_cast<CFloatValue*>(v.get())){std::cout<<",\"kind\":\"Float\",\"native_default\":\""<<std::format("{:08x}",std::bit_cast<uint32_t>(x->defaultVal()))<<'"';limits(x);}
  else if(auto*x=dynamic_cast<CStringValue*>(v.get())){
   std::cout<<",\"kind\":\"String\",\"native_default\":";quoted(x->defaultVal());
   auto fn=validator(x);std::cout<<",\"has_validator\":";boolean(bool(fn));
   if(fn){std::cout<<",\"validator_cases\":[";bool first=true;for(const auto&input:STRING_CASES){if(!first)std::cout<<',';first=false;std::cout<<'[';quoted(input);std::cout<<',';boolean(fn(input).has_value());std::cout<<']';}std::cout<<']';}
  }
  else if(auto*x=dynamic_cast<CColorValue*>(v.get()))std::cout<<",\"kind\":\"Color\",\"native_default\":null";
  else if(auto*x=dynamic_cast<CCssGapValue*>(v.get())){std::cout<<",\"kind\":\"CssGap\",\"native_default\":null,\"minimum\":";bound(x->options.min);std::cout<<",\"maximum\":";bound(x->options.max);}
  else if(auto*x=dynamic_cast<CVec2Value*>(v.get())){std::cout<<",\"kind\":\"Vec2\",\"native_default\":null,\"has_validator\":";boolean(bool(x->options.validator));}
  else if(auto*x=dynamic_cast<CFontWeightValue*>(v.get()))std::cout<<",\"kind\":\"FontWeight\",\"native_default\":null";
  else if(auto*x=dynamic_cast<CGradientValue*>(v.get()))std::cout<<",\"kind\":\"Gradient\",\"native_default\":null";
  else return 3;
  lua_options(Config::Lua::fromGenericValue(v).get());
  std::cout<<'}';
 }
 std::cout<<"],\"device_parent_names_hex\":[";first=true;
 for(const auto&name:Config::Values::CONFIG_DEVICE_VALUE_NAMES){if(!first)std::cout<<',';first=false;quoted(name);}
 std::cout<<"],\"device_values\":[";first=true;
 Config::Lua::SDeviceConfig device;
 std::map<std::string,const Config::Lua::ILuaConfigValue*> fields;
 for(const auto&[name,value]:device.values)fields[name]=value.get();
 for(const auto&[name,value]:fields){if(!first)std::cout<<',';first=false;std::cout<<"{\"field_hex\":";quoted(name);lua_options(value);std::cout<<'}';}
 std::cout<<"],\"runtime_registry_additions\":[";first=true;
 Config::Lua::CConfigManager bootstrap;bootstrap.capture_bootstrap();
 for(const auto&[name,value]:bootstrap.m_configValues){if(!first)std::cout<<',';first=false;std::cout<<"{\"lua_key_hex\":";quoted(name);lua_options(value.get());std::cout<<'}';}
 std::cout<<"]}\n";
}
'''


def strip_includes(source):
    return '\n'.join(line for line in source.splitlines() if not line.lstrip().startswith(('#include','#pragma')))+'\n'


def chunks(source):
    """Code positions plus opaque literals/comments; never interpret C++ values."""
    index=0
    while index<len(source):
        start=index
        if source.startswith('//',index):
            end=source.find('\n',index)
            index=len(source) if end<0 else end+1
            yield start,source[start:index],'comment'
        elif source.startswith('/*',index):
            index=source.index('*/',index+2)+2
            yield start,source[start:index],'comment'
        elif source.startswith('R"',index):
            opening=source.index('(',index+2)
            delimiter=source[index+2:opening]
            closing=')'+delimiter+'"'
            index=source.index(closing,opening+1)+len(closing)
            yield start,source[start:index],'literal'
        elif source[index] in '\"\'':
            quote=source[index];index+=1
            while source[index]!=quote:
                index+=2 if source[index]=='\\' else 1
            index+=1
            yield start,source[start:index],'literal'
        else:
            index+=1
            yield start,source[start:index],'code'


def normalized_cpp(source):
    return ''.join(text for _,text,kind in chunks(source) if kind=='literal' or kind=='code' and not text.isspace())


def registration_metadata(source):
    code_positions={index for index,_,kind in chunks(source) if kind=='code'}
    result={}
    for match in re.finditer(r'MS<(\w+)>\s*\(',source):
        if match.start() not in code_positions:
            continue
        start=match.end();stack=['(']
        for offset,text,kind in chunks(source[start:]):
            if kind!='code':
                continue
            if text in '([{':stack.append(text)
            elif text in ')]}':
                if stack.pop()!={')':'(',']':'[','}':'{'}[text]:
                    raise RuntimeError('Mismatched registration delimiters')
                if not stack:
                    break
        else:raise RuntimeError('Unterminated setting registration')
        # Split once more using stable offsets; commas inside literals/options
        # stay opaque and no C++ expression is evaluated in Python.
        body=source[start:start+offset];stack=[];arguments=[];segment=0
        for index,text,kind in chunks(body):
            if kind!='code':continue
            if text in '([{':stack.append(text)
            elif text in ')]}':stack.pop()
            elif text==',' and not stack:
                arguments.append(body[segment:index].strip());segment=index+1
        arguments.append(body[segment:].strip())
        if not 2<=len(arguments)<=4:
            raise RuntimeError('Review changed setting constructor arguments')
        identifier=json.loads(arguments[0])
        if identifier in result:raise RuntimeError('Duplicate source setting ID')
        result[identifier]=(match[1],arguments[3] if len(arguments)==4 else '{}')
    return result


def block(source, signature, suffix=''):
    start=source.index(signature)
    opening=source.index('{',start)
    depth=1
    for offset,text,kind in chunks(source[opening+1:]):
        if kind!='code':continue
        depth+=(text=='{')-(text=='}')
        if not depth:return source[start:opening+offset+2]+suffix+'\n'
    raise RuntimeError('Unterminated pinned C++ block')


def fetch(row):
    with urllib.request.urlopen(row['source'],timeout=30) as reply:data=reply.read()
    if hashlib.sha256(data).hexdigest()!=row['sha256']:raise RuntimeError('Pinned registration source differs')
    return (row['version'],row['path']),data.decode()


def child_limits():
    resource.setrlimit(resource.RLIMIT_AS,(256*1024*1024,)*2)
    resource.setrlimit(resource.RLIMIT_CPU,(3,)*2)
    resource.setrlimit(resource.RLIMIT_STACK,(1024*1024,)*2)
    resource.setrlimit(resource.RLIMIT_CORE,(0,)*2)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    manifest=(ROOT/'data/scalar-registration-sources.json').read_bytes()
    rows=json.loads(manifest)['files']
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:sources=dict(pool.map(fetch,rows))
    inventory=json.loads((ROOT/'docs/hyprland-feature-inventory.json').read_text())
    items={'baseline':inventory['scalar_settings'],'current':inventory['current_source_comparison']['scalar_settings']}
    choices=set(['','[[EMPTY]]','unknown','ADAPTIVE','adaptive ',' adaptive','1'])
    for entries in items.values():
        for entry in entries:
            metadata=entry.get('metadata_cpp','')
            if 'strChoice' in metadata:
                expression=metadata.split('strChoice(',1)[1].split(')',1)[0]
                choices.update(json.loads(s) for s in re.findall(r'"(?:\\.|[^"\\])*"',expression))
    case_expressions=[json.dumps(s) for s in sorted(choices)]
    case_expressions+=['std::string("adaptive\\0suffix",15)','std::string("\\xff",1)']
    output=[]
    with tempfile.TemporaryDirectory(prefix='hye-scalar-register-') as folder:
        temp=Path(folder)
        for version in ['baseline','current']:
            get=lambda path:sources[(version,path)]
            values=get('src/config/values/ConfigValues.cpp')
            metadata=registration_metadata(block(values,'std::vector<SP<IValue>> Values::getConfigValues()'))
            for entry in items[version]:
                kind,options=metadata[entry['id']]
                if kind!=entry['type'] or normalized_cpp(options)!=normalized_cpp(entry.get('metadata_cpp') or '{}'):
                    raise RuntimeError(f"Review source metadata drift for {version} {entry['id']}")
            if len(metadata)!=len(items[version]):raise RuntimeError('Unclassified source setting constructor')
            flags=sorted(set(re.findall(r'Supplementary::(REFRESH_[A-Z_]+)',values)))
            cpp=HEADER+''.join(f'constexpr PropRefreshBits {flag}=0;\n' for flag in flags)+'}\n'
            cpp+='\n'.join(line for line in get('src/macros.hpp').splitlines() if line.startswith('#define STRVAL_EMPTY '))+'\n'
            cpp+=strip_includes(get('src/config/values/types/IValue.hpp'))
            for kind in KINDS:
                header=get(f'src/config/values/types/{kind}Value.hpp')
                cpp+=strip_includes(header) if kind in PRIMITIVES else 'namespace Config::Values{'+block(header,f'struct S{kind}ValueOptions',';')+'}\n'
            cpp+=('#define BASE_PROPS(props) props.refresh\n' if version=='baseline' else '#define BASE_PROPS(props) props.refresh,props.deprecationNotice\n')+CAPTURE
            cpp+=strip_includes(get('src/config/values/ConfigValues.hpp'))
            cpp+=strip_includes(get('src/config/values/types/IValue.cpp'))
            for kind in PRIMITIVES:cpp+=strip_includes(get(f'src/config/values/types/{kind}Value.cpp'))
            cpp+='namespace Config::Lua {struct ILuaConfigValue{virtual ~ILuaConfigValue()=default;void setRefreshBits(Supplementary::PropRefreshBits){}void setDeprecated(const char*){}bool m_bSetByUser=false;};\n'
            fields={
                'Bool':'Config::BOOL m_default=false,m_data=false;',
                'Int':'Config::INTEGER m_default=0,m_data=0;std::optional<Config::INTEGER>m_min,m_max;std::optional<std::unordered_map<std::string,Config::INTEGER>>m_map;',
                'Float':'Config::FLOAT m_default=0,m_data=0;std::optional<Config::FLOAT>m_min,m_max;',
                'String':'Config::STRING m_default,m_data;StringValidator m_validator;',
                'Color':'Config::INTEGER m_default=0,m_data=0;',
                'CssGap':'Config::INTEGER m_default=0;GapData m_data;std::optional<int64_t>m_min,m_max;',
                'Vec2':'Config::VEC2 m_default,m_data;std::function<std::expected<void,std::string>(const Config::VEC2&)>m_validator;',
                'FontWeight':'Config::INTEGER m_default=400;WeightData m_data;',
                'Gradient':'CHyprColor m_default;GradientData m_data{CHyprColor{}};',
            }
            for kind in KINDS:
                header=get(f'src/config/lua/types/LuaConfig{kind}.hpp')
                name=f'CLuaConfig{kind}'
                declaration=header[header.index(name+'('):].split(';',1)[0]+';'
                cpp+=f'struct {name}:ILuaConfigValue{{'+fields[kind]+declaration+'};\n'
            cpp+='UP<ILuaConfigValue>fromGenericValue(SP<Config::Values::IValue>); }\nusing namespace Config;using namespace Config::Lua;\n'
            for kind in KINDS:cpp+=block(get(f'src/config/lua/types/LuaConfig{kind}.cpp'),f'CLuaConfig{kind}::CLuaConfig{kind}(')
            cpp+=block(get('src/config/lua/types/LuaConfigUtils.cpp'),'UP<ILuaConfigValue> Lua::fromGenericValue')
            manager=get('src/config/lua/ConfigManager.cpp')
            constructor=block(manager,'CConfigManager::CConfigManager()')
            extras=re.findall(r'm_configValues\["[^"]+"\]\s*=.*?;',constructor)
            if len(extras)!=1:raise RuntimeError('Review static registry additions')
            cpp+='namespace Config::Lua {struct CConfigManager{std::map<std::string,UP<ILuaConfigValue>>m_configValues;static std::string luaConfigValueName(const std::string&);void capture_bootstrap(){'+extras[0]+'}};CConfigManager*mgr(){static CConfigManager m;return &m;}struct SDeviceConfig{std::unordered_map<std::string,UP<ILuaConfigValue>>values;SDeviceConfig();};}\n'
            cpp+='#define RASSERT(condition,...) do{if(!(condition))std::abort();}while(0)\n'
            cpp+=block(manager,'std::string CConfigManager::luaConfigValueName')
            cpp+=block(manager,'SDeviceConfig::SDeviceConfig(')
            cpp+='using namespace Config;using namespace Config::Values;\n'+block(values,'std::vector<SP<IValue>> Values::getConfigValues()')
            cpp+='\nconst std::vector<std::string> STRING_CASES={'+','.join(case_expressions)+'};\n'+MAIN
            path=temp/f'{version}.cpp';path.write_text(cpp);binary=temp/version
            subprocess.run(['c++','-std=c++23',str(path),'-o',str(binary)],check=True,timeout=90)
            result=subprocess.run([str(binary)],capture_output=True,text=True,check=True,timeout=5,preexec_fn=child_limits)
            captured=json.loads(result.stdout)
            captured['values'].sort(key=lambda row:row['id_hex'])
            expected={row['id']:row['type'] for row in items[version]}
            actual={bytes.fromhex(row['id_hex']).decode():row['kind'] for row in captured['values']}
            if actual!=expected:raise RuntimeError(f'{version} original registrations differ from audit: missing={set(expected)-set(actual)}, extra={set(actual)-set(expected)}')
            output.append({'version':version,**captured})
            print(f'{version}: {len(actual)} original settings; {len(captured["device_values"])} device values; {len(captured["runtime_registry_additions"])} bootstrap additions',flush=True)
    result={'format':'hye-scalar-registration-fixtures-1','sources_sha256':hashlib.sha256(manifest).hexdigest(),'audit_inventory_sha256':hashlib.sha256((ROOT/'docs/hyprland-feature-inventory.json').read_bytes()).hexdigest(),'registration_metadata_matches_audit':True,
            'evidence_scope':'Original option structs, factory, registration function, four primitive value classes, all nine original Lua constructors, generic-to-Lua conversion, key normalization, device constructor and string validators. Inert config lookup/refresh/memory adapters and generic complex storage/derived caches; complex defaults and parsed Lua effects, runtime device availability and effects are not certified. Captured device values do not include separately handled tag/selector fields. Bootstrap additions execute only the isolated original registration statement, not the manager constructor or watcher.',
            'snapshots':output}
    encoded=json.dumps(result,indent=2)+'\n'
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text()!=encoded:raise SystemExit('Scalar registration observations differ')
    else:OUTPUT.write_text(encoded)


if __name__=='__main__':main()
