#!/usr/bin/env python3
"""Check primitive rule codecs against pinned upstream parsing functions.

Numeric/list/color parsing uses actual source and pinned Hyprutils sources.
Graphics gradient/color storage is inert: this does not render or run Hyprland.
"""
import argparse, concurrent.futures, hashlib, json, subprocess, tempfile, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
MANIFEST=json.loads((ROOT/'data/rule-codec-provenance.json').read_text())
CASES={
 'opacity':['','0.5','0.5 override','0.5 override 0.25','override opacity 0.5 override','0.5 0.25 0.75 override','0.5  0.25','0.5 0.25 1 1','+0.5','1e-50','1e-45','3.4028235e38','3.4028236e38'],
 'fullscreen':['2','2 3','0x2 0x3 ignored','2  3',' 2 3','2147483647 -2147483648','2147483648'],
 'border':['0xFF000000 0xFFFFFFFF','0xFF000000 45deg #fff 90deg','#ff000080','rgba(ff000080)','rgba(255, 0, 0, 0.5)','0deg 0xFFFFFFFF 90deg','0deg',' '.join(['0xFF000000']*11)+' 0deg'],
}
def block(source,signature,ending=''):
 start=source.index(signature);end=source.index('{',start)+1;depth=1
 while depth:
  depth+=(source[end]=='{')-(source[end]=='}');end+=1
 return source[start:end]+ending
HEADER=r'''
#include <algorithm>
#include <cassert>
#include <cmath>
#include <expected>
#include <format>
#include <iomanip>
#include <iostream>
#include <numbers>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>
#include <hyprutils/string/Numeric.hpp>
#include <hyprutils/string/String.hpp>
#include <hyprutils/string/VarList.hpp>
#include <hyprutils/string/VarList2.hpp>
using namespace Hyprutils::String;
template<class T,class U>T sc(U value){return static_cast<T>(value);}
struct CHyprColor{uint32_t hex;CHyprColor(uint64_t n):hex(n){}};
#ifndef __cpp_lib_string_view
#error string_view required
#endif
#if !defined(__cpp_lib_string_view) || __cpp_lib_string_view < 202403L
std::string operator+(std::string s,std::string_view v){s.append(v);return s;}
#endif
namespace Config{
struct CGradientValueData{
std::vector<CHyprColor> m_colors;float m_angle=0;
CGradientValueData()=default;CGradientValueData(CHyprColor c):m_colors{c}{}
void updateColorsOk(){} // renderer storage intentionally inert
};
namespace ParserUtils{std::expected<int64_t,std::string> parseColor(std::string_view);}
}
using namespace Config;
namespace Desktop::Types{
'''
MAIN=r'''
std::string q(std::string_view s){std::ostringstream o;o<<std::quoted(std::string{s});return o.str();}
std::string alpha(const Desktop::Types::SAlphaValue& a){return std::format(R"({{"amount":{},"override_value":{}}})",a.alpha,a.overridden?"true":"false");}
std::string gradient(const Config::CGradientValueData& g){std::string colors;for(auto c:g.m_colors){if(!colors.empty())colors+=",";colors+=std::to_string(c.hex);}return std::format(R"({{"colors":[{}],"degrees":{}}})",colors,std::lround(g.m_angle*180./std::numbers::pi));}
int main(){std::string kind,input;while(std::getline(std::cin,kind,'\t')&&std::getline(std::cin,input)){
std::cout<<"{\"kind\":"<<q(kind)<<",\"raw\":"<<q(input)<<",\"parsed\":";
if(kind=="opacity"){auto v=parseOpacityRule(input);if(!v)std::cout<<"null";else std::cout<<"{\"active\":"<<alpha(v->alpha)<<",\"inactive\":"<<alpha(v->alphaInactive)<<",\"fullscreen\":"<<alpha(v->alphaFullscreen)<<"}";}
else if(kind=="fullscreen"){auto v=parseFullscreenState(input);if(!v)std::cout<<"null";else std::cout<<"{\"internal\":"<<v->internal<<",\"client\":"<<(v->client?std::to_string(*v->client):"null")<<"}";}
else if(kind=="border" || kind=="border_public_string"){auto v=[&](){if(kind=="border_public_string"){if(auto col=Config::ParserUtils::parseColor(input);col){return parseBorderColorRule(std::format("0x{:08X} 0deg",sc<uint32_t>(*col)));}}return parseBorderColorRule(input);}();if(!v)std::cout<<"null";else{
std::cout<<"{\"active\":";
ACTIVE_OUTPUT
std::cout<<",\"inactive\":"<<(v->inactive?gradient(*v->inactive):"null")<<"}";
}}
else if(kind.starts_with("group:")){auto initial=static_cast<uint16_t>(std::stoi(kind.substr(6)));std::cout<<groupProbe(input,initial);}
std::cout<<"}\n";
}}
'''
CASES['border_public_string']=CASES['border']
for state in [0,128,64]:CASES['group:'+str(state)]=['set always barred lock always invade deny','new','override lock','unset lock','set group always','set  always','group','set always override lock always']
args=argparse.ArgumentParser();args.add_argument('--check',action='store_true');check=args.parse_args().check
with tempfile.TemporaryDirectory(prefix='hye-rule-codecs-') as temp:
 directory=Path(temp)
 def fetch(row):
  cache=ROOT/'.dev/rule-codec-reference'/f'{row["version"]}-{row["path"].replace("/","_")}'
  data=cache.read_bytes() if cache.is_file() else urllib.request.urlopen(f'https://raw.githubusercontent.com/{row["repo"]}/{row["commit"]}/{row["path"]}',timeout=20).read()
  if hashlib.sha256(data).hexdigest()!=row['sha256']:raise RuntimeError('Pinned primitive parser source hash mismatch')
  return row,data
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:files=list(pool.map(fetch,MANIFEST['source_files']))
 source={}
 for row,data in files:
  if row['version']=='hyprutils':
   path=directory/'hyprutils'/row['path'];path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
  else:source[(row['version'],Path(row['path']).name)]=data.decode()
 fixtures=[]
 for version in ['baseline','current']:
  rule=source[(version,'WindowRule.cpp')];types=source[(version,'WindowRule.hpp')];parser=source[(version,'ParserUtils.cpp')]
  cpp=HEADER+block(source[(version,'OverridableVar.hpp')],'struct SAlphaValue',';')+'}\nusing namespace Desktop::Types;\nnamespace Types=Desktop::Types;\n'
  for name in ['SFullscreenStateRule','SOpacityRule','SBorderColorRule']:cpp+=block(types,'struct '+name,';')
  cpp+=block(rule,'static const char* numericParseError')
  cpp+=block(parser,'static std::expected<uint64_t, std::string> parseHex')
  cpp+=block(parser,'std::expected<int64_t, std::string> ParserUtils::parseColor')
  for name in ['parseBorderColorToken','parseFullscreenState','parseOpacityRule','parseBorderColorRule']:
   start=rule.index(name+'(');signature_start=rule.rfind('static ',0,start);cpp+=block(rule,rule[signature_start:start+len(name)])
  output='std::cout<<(v->active?gradient(*v->active):"null");' if version=='current' else 'std::cout<<gradient(v->active);'
  cpp+='namespace Desktop::View{'+block(source[('current','WindowGroupMembership.hpp')],'enum eGroupRules',';')+'uint16_t parseGroupRules(std::string_view,uint16_t);}'
  cpp+='\nusing namespace Desktop::View;\n#define LOG(...) ((void)0)\n'
  cpp+='struct QuietLogger{template<class...T>void log(T...){}};namespace Log{inline QuietLogger quiet;inline QuietLogger* logger=&quiet;constexpr int ERR=0;}\n'
  if version=='current':
   cpp+=block(source[(version,'WindowGroupMembership.cpp')],'uint16_t Desktop::View::parseGroupRules')
   cpp+='uint16_t groupProbe(std::string_view raw,uint16_t initial){return Desktop::View::parseGroupRules(raw,initial);}'
  else:
   cpp+='uint16_t groupProbe(std::string_view raw,uint16_t initial){uint16_t m_groupRules=initial;struct Applicator{struct{std::string group;}static_;}app;app.static_.group=raw;auto* m_ruleApplicator=&app;'
   cpp+=block(source[(version,'Window.cpp')],'if (!m_ruleApplicator->static_.group.empty())')
   cpp+='return m_groupRules;}'
  cpp+=MAIN.replace('ACTIVE_OUTPUT',output)
  path=directory/'probe.cpp';path.write_text(cpp);binary=directory/'probe'
  library=directory/'hyprutils'
  subprocess.run(['c++','-std=c++23','-O0','-I'+str(library/'include'),str(path),str(library/'src/string/VarList.cpp'),str(library/'src/string/VarList2.cpp'),str(library/'src/string/String.cpp'),'-o',str(binary)],check=True,timeout=45)
  inputs=''.join(kind+'\t'+raw+'\n' for kind,values in CASES.items() for raw in values)
  result=subprocess.run([str(binary)],input=inputs,text=True,capture_output=True,check=True,timeout=3)
  values=[dict(json.loads(line),version=version) for line in result.stdout.splitlines()]
  fixtures.extend(values);print(version+': '+str(len(values))+' actual primitive parser observations')
 content=json.dumps({'format':'hye-rule-codec-fixtures-1','meaning':'Pinned primitive parser observations with inert gradient storage; no compositor behavior certificate.','cases':fixtures},indent=2)+'\n'
 path=ROOT/'data/rule-codec-fixtures.json'
 if check:
  if path.read_text()!=content:raise RuntimeError('Primitive parser fixture differs; review the source/profile change')
 else:path.write_text(content)
