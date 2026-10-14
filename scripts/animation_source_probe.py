#!/usr/bin/env python3
"""Execute pinned Lua animation bindings, numeric descriptors, style validation and tree propagation.

Compositor curve registries are inert storage. String descriptor is a literal-only fixture adapter;
shared/weak pointers use std equivalents. This is source conformance, not rendered preview.
"""
import argparse, concurrent.futures, hashlib, json, shlex, subprocess, tempfile, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
MANIFEST=json.loads((ROOT/'data/animation-semantics.json').read_text())
def block(source,signature,ending=''):
 start=source.index(signature);end=source.index('{',start)+1;depth=1
 while depth:
  depth+=(source[end]=='{')-(source[end]=='}');end+=1
 return source[start:end]+ending
HEADER=r'''
#include <lua.hpp>
#include <algorithm>
#include <format>
#include <functional>
#include <iomanip>
#include <iostream>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <unordered_map>
#include <typeinfo>
namespace Hyprutils::Memory {
template<class T>using CSharedPointer=std::shared_ptr<T>;
template<class T>class CWeakPointer:public std::weak_ptr<T>{public:
using std::weak_ptr<T>::weak_ptr;CWeakPointer()=default;CWeakPointer(const std::shared_ptr<T>&s):std::weak_ptr<T>(s){};
T* operator->()const{return this->lock().get();}explicit operator bool()const{return !this->expired();}
bool operator==(const std::shared_ptr<T>& s)const{return this->lock()==s;}};
template<class T>auto makeShared(){return std::make_shared<T>();}
}
using namespace Hyprutils::Memory;
#define SP CSharedPointer
#define WP CWeakPointer
struct Vector2D{double x,y;Vector2D(double x,double y):x(x),y(y){}};
class CScopeGuard{std::function<void()> f;public:CScopeGuard(std::function<void()>fn):f(fn){}~CScopeGuard(){f();}};
namespace Config {
using FLOAT=float;using BOOL=bool;
enum{PARSE_ERROR_OK,PARSE_ERROR_BAD_TYPE,PARSE_ERROR_BAD_VALUE,PARSE_ERROR_OUT_OF_RANGE};
struct SParseError{int errorCode=0;std::string message;};
namespace Lua{
struct CLuaConfigFloat{float m_default,m_data;std::optional<float>m_min,m_max;bool m_bSetByUser=false;
CLuaConfigFloat(float def,std::optional<float>min={},std::optional<float>max={}):m_default(def),m_data(def),m_min(min),m_max(max){};
SParseError parse(lua_State*);const float& parsed(){return m_data;}};
struct CLuaConfigBool{bool m_data,m_bSetByUser=false;CLuaConfigBool(bool def):m_data(def){};SParseError parse(lua_State*);bool parsed(){return m_data;}};
struct CLuaConfigString{std::string value;CLuaConfigString(const char*s):value(s){};
SParseError parse(lua_State* L){if(lua_type(L,-1)!=LUA_TSTRING)return{PARSE_ERROR_BAD_TYPE,"literal string adapter"};size_t n=0;auto p=lua_tolstring(L,-1,&n);value.assign(p,n);return{};}const std::string& parsed(){return value;}};
namespace Internal{
int configError(lua_State*L,const std::string& s){return luaL_error(L,"%s",s.c_str());}
bool hasTableField(lua_State*L,int idx,const char*field);
}
}
}
using namespace Config;using namespace Config::Lua;
'''
MANAGER=r'''
struct CHyprAnimationManager {
std::set<std::string> beziers{"default","linear"},springs{"default"};
void addBezierWithName(const std::string&name,Vector2D,Vector2D){beziers.insert(name);}
void addSpringWithName(const std::string&name,Hyprutils::Animation::SSpringCurve){springs.insert(name);}
bool bezierExists(const std::string&n){return beziers.contains(n);}bool springExists(const std::string&n){return springs.contains(n);}
std::string styleValidInConfigVar(const std::string&,const std::string&);
};
namespace Animation{CHyprAnimationManager* mgr(){static CHyprAnimationManager m;return &m;}}
'''
MAIN=r'''
std::string q(const std::string&s){std::ostringstream out;out<<std::quoted(s);return out.str();}
int main(){auto* L=luaL_newstate();luaL_openlibs(L);lua_newtable(L);lua_pushcfunction(L,hlCurve);lua_setfield(L,-2,"curve");lua_pushcfunction(L,hlAnimation);lua_setfield(L,-2,"animation");lua_setglobal(L,"hl");
std::string line;while(std::getline(std::cin,line)){
Config::animationTree()->reset();*Animation::mgr()=CHyprAnimationManager{};
auto status=luaL_dostring(L,line.c_str());std::cout<<"{\"accepted\":"<<(status==LUA_OK?"true":"false")<<",\"tree\":[";
std::map<std::string,SP<Hyprutils::Animation::SAnimationPropertyConfig>> sorted(Config::animationTree()->m_animationTree.getFullConfig().begin(),Config::animationTree()->m_animationTree.getFullConfig().end());bool comma=false;
for(auto&[name,node]:sorted){if(comma)std::cout<<",";comma=true;auto parent=node->pParentAnimation.lock();auto values=node->pValues.lock();std::string origin,parentName;for(auto&[n,p]:sorted){if(p==values)origin=n;if(p==parent)parentName=n;}
std::cout<<"{\"name\":"<<q(name)<<",\"parent\":"<<(parent==node?"null":q(parentName))<<",\"origin\":"<<q(origin)<<",\"overridden\":"<<(node->overridden?"true":"false")<<",\"enabled\":"<<(values->internalEnabled?"true":"false")<<",\"speed\":"<<std::setprecision(17)<<values->internalSpeed<<",\"curve\":"<<q(values->internalBezier)<<",\"style\":"<<q(values->internalStyle)<<"}";}
std::cout<<"]}\n";lua_settop(L,0);
}lua_close(L);}
'''
CASES=[('defaults',''),('missing-enabled',"hl.animation{leaf='windows',speed=8,bezier='default'}"),('disabled-ignored',"hl.animation{leaf='windows',enabled=false,speed=-5,spring='missing',style='unknown'}"),('ancestor',"hl.animation{leaf='global',enabled=true,speed=3,bezier='linear'};hl.animation{leaf='windows',enabled=true,speed=5,bezier='default',style='slide'};hl.animation{leaf='windowsIn',enabled=false}"),('bezier-bool',"hl.curve('ease',{type='bezier',points={{false,1},{0.25,1}}});hl.animation{leaf='windows',enabled=1,speed='5',bezier='ease'}"),('bezier-before',"hl.animation{leaf='windows',enabled=true,speed=5,bezier='later'};hl.curve('later',{type='bezier',points={{0,0},{1,1}}})"),('dual-reference',"hl.animation{leaf='windows',enabled=true,speed=5,bezier='linear',spring='missing'}"),('spring-old',"hl.curve('s',{type='spring',stiffness=250,dampening=25,mass=1});hl.animation{leaf='windows',enabled=true,speed=5,spring='s'}"),('spring-new',"hl.curve('s',{type='spring',stiffness=250,damping=25,mass=1});hl.animation{leaf='windows',enabled=true,speed=5,spring='s'}"),('spring-half',"hl.curve('s',{type='spring',stiffness=250,dampening=25,mass=0.5})"),('spring-f32-rounding',"hl.curve('s',{type='spring',stiffness=250,dampening=25,mass=0.50000000001})"),('spring-bool',"hl.curve('s',{type='spring',stiffness=true,dampening=25,mass=1})"),('speed-underflow',"hl.animation{leaf='windows',enabled=true,speed=1e-50,bezier='default'}"),('bezier-edge',"hl.curve('s',{type='bezier',points={{-1,2},{0,1}}})"),('bezier-outside',"hl.curve('s',{type='bezier',points={{-1.01,2},{0,1}}})")]
for leaf in ['windows','layers','workspaces','borderangle','fade']:
 for style in ['', 'slide', 'slidefade 80%', 'popin 80%', 'popin 80% ', 'popin  80%', 'popin -2147483648%', 'popin 2147483648%', 'popin 80%é', 'slide 80%é', 'loop','once','gnome','fade']:
  CASES.append((f'style:{leaf}:{style}',f'hl.animation{{leaf={json.dumps(leaf)},enabled=true,speed=5,bezier="default",style={json.dumps(style,ensure_ascii=False)}}}'))
parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
with tempfile.TemporaryDirectory(prefix='hye-animation-probe-') as temp:
 def fetch(row):
  cache=ROOT/'.dev/animation-reference'/f'{row["version"]}-{row["path"].replace("/","_")}'
  data=cache.read_bytes() if cache.exists() else urllib.request.urlopen(f'https://raw.githubusercontent.com/{row["repo"]}/{row["commit"]}/{row["path"]}',timeout=20).read()
  if hashlib.sha256(data).hexdigest()!=row['sha256']:raise RuntimeError('Pinned animation source hash mismatch: '+row['path'])
  if 'git_blob' in row and hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()!=row['git_blob']:raise RuntimeError('Pinned animation git blob mismatch')
  return (row['version'],Path(row['path']).name),data.decode()
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:source=dict(pool.map(fetch,MANIFEST['source_files']))
 results=[]
 for version in ['baseline','current']:
  cpp=HEADER
  cpp+='namespace Hyprutils::Animation{'+block(source[('hyprutils','AnimationConfig.hpp')],'struct SAnimationPropertyConfig',';')+block(source[('hyprutils','AnimationConfig.hpp')],'class CAnimationConfigTree',';')+block(source[('hyprutils','AnimationManager.hpp')],'struct SSpringCurve',';')+'}\nusing namespace Hyprutils::Animation;\n'
  for signature in ['void CAnimationConfigTree::createNode','bool CAnimationConfigTree::nodeExists','void CAnimationConfigTree::setConfigForNode','SP<SAnimationPropertyConfig> CAnimationConfigTree::getConfig','const std::unordered_map<std::string, SP<SAnimationPropertyConfig>>& CAnimationConfigTree::getFullConfig','void CAnimationConfigTree::setAnimForChildren']:cpp+=block(source[('hyprutils','AnimationConfig.cpp')],signature)
  tree=block(source[(version,'AnimationTree.hpp')],'class CAnimationTreeController',';').replace('      private:','      public:')
  cpp+='namespace Config{'+tree+'CAnimationTreeController* animationTree(){static CAnimationTreeController t;return &t;}}\n'
  for signature in ['CAnimationTreeController::CAnimationTreeController','void CAnimationTreeController::reset','void CAnimationTreeController::setConfigForNode','bool CAnimationTreeController::nodeExists']:cpp+=block(source[(version,'AnimationTree.cpp')],signature)
  cpp+=MANAGER+block(source[(version,'AnimationManager.cpp')],'std::string CHyprAnimationManager::styleValidInConfigVar')
  cpp+=block(source[(version,'LuaConfigFloat.cpp')],'SParseError CLuaConfigFloat::parse')+block(source[(version,'LuaConfigBool.cpp')],'SParseError CLuaConfigBool::parse')
  cpp+='namespace Config::Lua::Internal{template<typename T>'+block(source[(version,'LuaBindingsInternal.hpp')],'SParseError parseTableField')+'}\n'
  cpp+=block(source[(version,'LuaBindingsInternal.cpp')],'bool Internal::hasTableField')
  cpp+=block(source[(version,'LuaBindingsConfigRules.cpp')],'static int hlCurve')+block(source[(version,'LuaBindingsConfigRules.cpp')],'static int hlAnimation')+MAIN
  path=Path(temp)/'probe.cpp';path.write_text(cpp);binary=Path(temp)/'probe'
  flags=shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','lua5.4'],text=True))
  subprocess.run(['c++','-std=c++23','-O0',str(path),*flags,'-o',str(binary)],check=True,timeout=45)
  result=subprocess.run([str(binary)],input='\n'.join(s for _,s in CASES)+'\n',text=True,capture_output=True,check=True,timeout=4)
  observations=[json.loads(line) for line in result.stdout.splitlines()];assert len(observations)==len(CASES)
  defaults={r['name']:r for r in observations[0]['tree']}
  for (name,code),obs in zip(CASES,observations):
   if name!='defaults':obs['tree']=[r for r in obs['tree'] if r!=defaults[r['name']]]
   results.append(dict(version=version,case=name,source=code,**obs))
  print(f'{version}: {len(observations)} actual animation binding/tree observations')
 content=json.dumps({'format':'hye-animation-source-fixtures-1','meaning':'Actual pinned animation binding/numeric/style/tree functions with inert curve registry, literal-string adapter and std pointer adapter; no renderer or compositor behavior certificate.','cases':results},indent=2,ensure_ascii=False)+'\n'
 path=ROOT/'data/animation-source-fixtures.json'
 if args.check:
  if path.read_text()!=content:raise RuntimeError('Animation observations changed; inspect pinned sources/profile')
 else:path.write_text(content)
