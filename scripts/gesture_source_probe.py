#!/usr/bin/env python3
"""Pinned gesture registration, descriptors and callback payload observations with inert actions."""
import argparse,concurrent.futures,hashlib,json,shlex,subprocess,tempfile,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
MANIFEST=json.loads((ROOT/'data/gesture-semantics.json').read_text())
def block(source,signature,ending=''):
 start=source.index(signature);end=source.index('{',start)+1;depth=1
 while depth:depth+=(source[end]=='{')-(source[end]=='}');end+=1
 return source[start:end]+ending
HEADER=r'''
#include <lua.hpp>
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <expected>
#include <format>
#include <functional>
#include <iomanip>
#include <iostream>
#include <map>
#include <memory>
#include <optional>
#include <ranges>
#include <sstream>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>
#define UP std::unique_ptr
#define SP std::shared_ptr
template<class T,class...Args>auto makeUnique(Args&&...a){return std::make_unique<T>(std::forward<Args>(a)...);}
template<class T,class...Args>auto makeShared(Args&&...a){return std::make_shared<T>(std::forward<Args>(a)...);}
struct Vector2D {double x=0,y=0;double size()const{return std::hypot(x,y);}};
namespace Config{
using INTEGER=int64_t;using FLOAT=float;using BOOL=bool;
enum{PARSE_ERROR_OK,PARSE_ERROR_BAD_TYPE,PARSE_ERROR_BAD_VALUE,PARSE_ERROR_OUT_OF_RANGE};
struct SParseError{int errorCode=0;std::string message;};
namespace Lua{
struct CLuaConfigInt{int64_t m_data;std::optional<int64_t>m_min,m_max;std::optional<std::unordered_map<std::string,int64_t>>m_map;bool m_bSetByUser=false;CLuaConfigInt(int64_t n,int64_t min,int64_t max):m_data(n),m_min(min),m_max(max){};SParseError parse(lua_State*);const int64_t& parsed(){return m_data;}};
struct CLuaConfigFloat{float m_data;std::optional<float>m_min,m_max;bool m_bSetByUser=false;CLuaConfigFloat(float n,float min,float max):m_data(n),m_min(min),m_max(max){};SParseError parse(lua_State*);const float& parsed(){return m_data;}};
struct CLuaConfigBool{bool m_data,m_bSetByUser=false;CLuaConfigBool(bool n):m_data(n){};SParseError parse(lua_State*);bool parsed(){return m_data;}};
struct CLuaConfigString{std::string m_data;bool m_bSetByUser=false;std::function<std::expected<void,std::string>(const std::string&)>m_validator;CLuaConfigString(const char*s):m_data(s){};SParseError parse(lua_State*);const std::string& parsed(){return m_data;}};
namespace Internal {inline std::vector<std::string> errors;int configError(lua_State*,const std::string&s){errors.push_back(s);return 0;}
template<class...Args>int configError(lua_State*L,std::format_string<Args...>fmt,Args&&...args){return configError(L,std::format(fmt,std::forward<Args>(args)...));}}
class CConfigManager {public:static constexpr int LUA_TIMEOUT_EVENT_CALLBACK_MS=1000;lua_State* L=nullptr;std::vector<int> refs;
void registerLuaRef(int ref){refs.push_back(ref);}
template<class P>void callLuaFn(int ref,P push,int,const char*){if(ref==LUA_NOREF || ref==LUA_REFNIL)return;lua_rawgeti(L,LUA_REGISTRYINDEX,ref);int n=push(L);if(lua_pcall(L,n,0,0)!=LUA_OK){Internal::errors.emplace_back(lua_tostring(L,-1));lua_pop(L,1);}}
void callLuaFn(int ref){callLuaFn(ref,[](lua_State*){return 0;},0,"");}
};inline CConfigManager manager;inline CConfigManager* mgr(){return &manager;}
}}
using namespace Config;using namespace Config::Lua;
'''
STUBS=r'''
struct FixtureBuiltin: ITrackpadGesture {std::string tag;std::vector<std::string>args;FixtureBuiltin(std::string name,std::vector<std::string>a={}):tag(name),args(a){};void update(const STrackpadGestureUpdate&)override{};void end(const STrackpadGestureEnd&)override{}};
'''
MAIN=r'''
std::string q(const std::string&s){std::ostringstream o;o<<std::quoted(s);return o.str();}
void jsonValue(lua_State*L,int index,int depth=0){index=lua_absindex(L,index);if(depth>8){std::cout<<"null";return;}switch(lua_type(L,index)){
case LUA_TNIL:std::cout<<"null";break;case LUA_TBOOLEAN:std::cout<<(lua_toboolean(L,index)?"true":"false");break;case LUA_TNUMBER:if(lua_isinteger(L,index)){std::cout<<lua_tointeger(L,index);}else{std::ostringstream out;out<<std::setprecision(17)<<lua_tonumber(L,index);auto raw=out.str();std::cout<<raw;if(raw.find_first_of(".eE")==std::string::npos)std::cout<<".0";}break;case LUA_TSTRING:std::cout<<q(lua_tostring(L,index));break;
case LUA_TTABLE:{auto n=lua_rawlen(L,index);bool comma=false;if(n){std::cout<<"[";for(size_t i=1;i<=n;++i){if(comma)std::cout<<",";comma=true;lua_rawgeti(L,index,i);jsonValue(L,-1,depth+1);lua_pop(L,1);}std::cout<<"]";}else{std::cout<<"{";lua_pushnil(L);while(lua_next(L,index)){if(comma)std::cout<<",";comma=true;std::cout<<q(lua_tostring(L,-2))<<":";jsonValue(L,-1,depth+1);lua_pop(L,1);}std::cout<<"}";}break;}default:std::cout<<"null";
}}
int main(){std::string line;while(std::getline(std::cin,line)){
g_pTrackpadGestures->clearGestures();auto* L=luaL_newstate();luaL_openlibs(L);Config::Lua::manager={};Config::Lua::manager.L=L;Internal::errors.clear();
lua_newtable(L);lua_pushcfunction(L,hlGesture);lua_setfield(L,-2,"gesture");lua_setglobal(L,"hl");
luaL_dostring(L,R"(__trace={};hl.dsp={no_op=function() table.insert(__trace,'prepare');return function() table.insert(__trace,'dispatch') end end};hl.dispatch=function(f) return f() end)");
auto status=luaL_dostring(L,line.c_str());bool accepted=status==LUA_OK && Internal::errors.empty();if(status!=LUA_OK)lua_pop(L,1);
std::cout<<"{\"accepted\":"<<(accepted?"true":"false")<<",\"errors\":"<<Internal::errors.size()<<",\"registered\":[";bool comma=false;
for(auto&g:g_pTrackpadGestures->m_gestures){if(comma)std::cout<<",";comma=true;std::string tag="callbacks";std::vector<std::string>args;if(auto*b=dynamic_cast<FixtureBuiltin*>(g->gesture.get())){tag=b->tag;args=b->args;}
std::cout<<"{\"fingers\":"<<g->fingerCount<<",\"direction\":"<<q(g_pTrackpadGestures->stringForDir(g->direction))<<",\"modifiers\":"<<static_cast<uint32_t>(g->modMask)<<",\"scale\":"<<std::setprecision(17)<<g->deltaScale<<",\"disable_inhibit\":"<<(g->disableInhibit?"true":"false")<<",\"action\":"<<q(tag)<<",\"arguments\":[";for(size_t i=0;i<args.size();++i){if(i)std::cout<<",";std::cout<<q(args[i]);}std::cout<<"]}";
if(dynamic_cast<CLuaFunctionGesture*>(g->gesture.get())){
 if(g->direction==TRACKPAD_GESTURE_DIR_PINCH || g->direction==TRACKPAD_GESTURE_DIR_PINCH_IN || g->direction==TRACKPAD_GESTURE_DIR_PINCH_OUT){IPointer::SPinchUpdateEvent e{123,static_cast<uint32_t>(g->fingerCount),{-12,5},0.8,10};IPointer::SPinchEndEvent end{124,true};g->gesture->begin({.pinch=&e,.direction=TRACKPAD_GESTURE_DIR_PINCH_IN,.scale=g->deltaScale});g->gesture->update({.pinch=&e,.direction=TRACKPAD_GESTURE_DIR_PINCH_IN,.scale=g->deltaScale});g->gesture->end({.pinch=&end,.direction=TRACKPAD_GESTURE_DIR_PINCH_IN,.scale=g->deltaScale});}
 else{IPointer::SSwipeUpdateEvent e{123,static_cast<uint32_t>(g->fingerCount),{-12,5}};IPointer::SSwipeEndEvent end{124,true};g->gesture->begin({.swipe=&e,.direction=TRACKPAD_GESTURE_DIR_LEFT,.scale=g->deltaScale});g->gesture->update({.swipe=&e,.direction=TRACKPAD_GESTURE_DIR_LEFT,.scale=g->deltaScale});g->gesture->end({.swipe=&end,.direction=TRACKPAD_GESTURE_DIR_LEFT,.scale=g->deltaScale});}
}}
std::cout<<"],\"trace\":";lua_getglobal(L,"__trace");jsonValue(L,-1);lua_pop(L,1);std::cout<<",\"callback_errors\":"<<Internal::errors.size()<<"}\n";lua_close(L);
}}
'''
CASES=[]
for fingers in ['2','9','1','10','3.0',"'3'",'true']:CASES.append(('fingers:'+fingers,f"hl.gesture{{fingers={fingers},direction='horizontal',action='workspace'}}"))
for scale in ['0.1','0.10000000149011612','1','10','10.0000001','true','false',"'0.1'","'1'",'1e-50','-0.1','-10','0.099999999','-0.099999999']:CASES.append(('scale:'+scale,f"hl.gesture{{fingers=3,direction='horizontal',action='workspace',scale={scale}}}"))
for direction in ['SWIPE','l','r','top','t','bottom','b','horiz','vert','pinch','zoomin','zoomout','unknown']:CASES.append(('direction:'+direction,f"hl.gesture{{fingers=3,direction='{direction}',action='workspace'}}"))
for mods in ['SHIFT CONTROL ALT MOD2 MOD3 META MOD5 CAPS','notWINd','shift_alt','ignored','']:
 CASES.append(('modifiers:'+mods,f"hl.gesture{{fingers=3,direction='left',action='workspace',mods='{mods}'}}"))
for action in ['workspace','resize','move','special','close','float','fullscreen','cursor_zoom','cursorZoom','scroll_move','unknown']:
 CASES.append(('action:'+action,f"hl.gesture{{fingers=3,direction='left',action='{action}',mode='toggle',zoom_level=2.0,workspace_name='scratch'}}"))
for a,b in [('swipe','left'),('left','swipe'),('horizontal','right'),('right','horizontal'),('vertical','down'),('up','vertical'),('pinch','pinchin'),('pinchin','pinchout'),('pinchin','pinch')]:
 CASES.append((f'overlap:{a}:{b}',f"hl.gesture{{fingers=3,direction='{a}',action='workspace'}};hl.gesture{{fingers=3,direction='{b}',action='workspace',scale=2,disable_inhibit=true}}"))
for fields in ['',',scale=2',',mods="CTRL"',',disable_inhibit=true',',fingers=4']:
 identity='fingers=3,direction="left",action="unset"'+fields
 # fingers mutation cannot use duplicate table keys.
 if fields==',fingers=4':identity='fingers=4,direction="left",action="unset"'
 CASES.append(('unset:'+fields,'hl.gesture{fingers=3,direction="left",action="workspace"};hl.gesture{'+identity+'}'))
CASES.append(('errors-do-not-stop-later-declarations','hl.gesture{fingers=3.0,direction="left",action="workspace"};hl.gesture{fingers=4,direction="right",action="move"}'))
CASES.append(('captured-and-late-preparation','local saved=hl.dsp.no_op();hl.gesture{fingers=3,direction="left",action={start=function(e) hl.dispatch(saved) end,update=function()end,finish=function()hl.dispatch(hl.dsp.no_op())end}}'))
CASES.append(('legacy-end-has-no-payload','hl.gesture{fingers=3,direction="left",action=function(e)table.insert(__trace,e==nil and "nil-event" or e)end}'))
for direction in ['left','pinchin']:
 CASES.append(('payloads:'+direction,f"hl.gesture{{fingers=3,direction='{direction}',action={{start=function(e)table.insert(__trace,e)end,update=function(e)table.insert(__trace,e)end,finish=function(e)table.insert(__trace,e)end}}}}"))
CASES += [('empty-callback-table','hl.gesture{fingers=3,direction="left",action={}}'),('ignored-end-spelling','hl.gesture{fingers=3,direction="left",action={["end"]=function()end}}'),('invalid-callback-value','hl.gesture{fingers=3,direction="left",action={finish=true}}')]
parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
with tempfile.TemporaryDirectory(prefix='hye-gesture-probe-') as temp:
 def fetch(row):
  cache=ROOT/'.dev/gesture-reference'/f'{row["version"]}-{row["path"].replace("/","_")}'
  data=cache.read_bytes() if cache.exists() else urllib.request.urlopen(f'https://raw.githubusercontent.com/{row["repo"]}/{row["commit"]}/{row["path"]}',timeout=25).read()
  if hashlib.sha256(data).hexdigest()!=row['sha256'] or hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()!=row['git_blob']:raise RuntimeError('Pinned gesture source identity differs: '+row['path'])
  return (row['version'],Path(row['path']).name),data.decode()
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:source=dict(pool.map(fetch,MANIFEST['source_files']))
 results=[]
 for version in ['baseline','current']:
  cpp=HEADER
  cpp+='class IPointer{public:'
  for name in ['SSwipeBeginEvent','SSwipeUpdateEvent','SSwipeEndEvent','SPinchBeginEvent','SPinchUpdateEvent','SPinchEndEvent']:cpp+=block(source[(version,'IPointer.hpp')],'struct '+name,';')
  cpp+='};\n'+block(source[(version,'GestureTypes.hpp')],'enum eTrackpadGestureDirection',';')
  cpp+=block(source[(version,'ITrackpadGesture.hpp')],'class ITrackpadGesture',';')
  cpp+='void ITrackpadGesture::begin(const STrackpadGestureBegin&e){m_scale=e.scale;}float ITrackpadGesture::distance(const STrackpadGestureBegin&){return 0;}float ITrackpadGesture::distance(const STrackpadGestureUpdate&){return 0;}bool ITrackpadGesture::isDirectionSensitive(){return false;}\n'+STUBS
  for name,tag,arity in [('CWorkspaceSwipeGesture','workspace',0),('CResizeTrackpadGesture','resize',0),('CMoveTrackpadGesture','move',0),('CSpecialWorkspaceGesture','special',1),('CCloseTrackpadGesture','close',0),('CFloatTrackpadGesture','float',1),('CFullscreenTrackpadGesture','fullscreen',1),('CCursorZoomTrackpadGesture','cursor_zoom',2),('CScrollMoveTrackpadGesture','scroll_move',0)]:
   parameters=','.join('const std::string& '+c for c in ['a','b'][:arity]);values=','.join(['a','b'][:arity]);cpp+=f'class {name}:public FixtureBuiltin{{public:{name}({parameters}):FixtureBuiltin("{tag}",{{{values}}}){{}}}};\n'
  cpp+=block(source[(version,'LuaFunctionGesture.hpp')],'class CLuaFunctionGesture',';')
  callbacks=source[(version,'LuaFunctionGesture.cpp')]
  start=callbacks.index('static const char* directionToString');cpp+=callbacks[start:]
  modifiers='''namespace Input{enum eKeyboardModifiers:uint32_t{HL_MODIFIER_NONE=0,HL_MODIFIER_SHIFT=1,HL_MODIFIER_CAPS=2,HL_MODIFIER_CTRL=4,HL_MODIFIER_ALT=8,HL_MODIFIER_MOD2=16,HL_MODIFIER_MOD3=32,HL_MODIFIER_META=64,HL_MODIFIER_MOD5=128};using ModifierMask=uint32_t;}using namespace Input;\n'''
  cpp+=modifiers
  if version=='baseline':
   cpp+='class CKeybindManager{public:uint32_t stringToModMask(std::string);};inline CKeybindManager keybinds;inline auto* g_pKeybindManager=&keybinds;'+block(source[(version,'KeybindManager.cpp')],'uint32_t CKeybindManager::stringToModMask')
  else:
   resolver=source[(version,'Resolver.cpp')];cpp+=block(resolver,'static constexpr std::pair<std::string_view, Input::eKeyboardModifiers> MODIFIER_NAMES[]',';');cpp+='namespace Keybinds{Input::ModifierMask modMaskFromString(std::string);}'+block(resolver,'Input::ModifierMask Keybinds::modMaskFromString')
  cpp+=block(source[(version,'TrackpadGestures.hpp')],'class CTrackpadGestures',';').replace('  private:','  public:')
  cpp+='inline auto g_pTrackpadGestures=makeUnique<CTrackpadGestures>();\n'
  gestures=source[(version,'TrackpadGestures.cpp')]
  for signature in ['void CTrackpadGestures::clearGestures','eTrackpadGestureDirection CTrackpadGestures::dirForString','const char* CTrackpadGestures::stringForDir','std::expected<void, std::string> CTrackpadGestures::addGesture','std::expected<void, std::string> CTrackpadGestures::removeGesture']:cpp+=block(gestures,signature)
  for name in ['Int','Float','Bool','String']:cpp+=block(source[(version,'LuaConfig'+name+'.cpp')],'SParseError CLuaConfig'+name+'::parse')
  cpp+='namespace Config::Lua::Internal{template<typename T>'+block(source[(version,'LuaBindingsInternal.hpp')],'SParseError parseTableField')+'}\n'
  cpp+=block(source[(version,'LuaBindingsConfigRules.cpp')],'static int hlGesture')+MAIN
  path=Path(temp)/'probe.cpp';path.write_text(cpp);binary=Path(temp)/'probe'
  flags=shlex.split(subprocess.check_output(['pkg-config','--cflags','--libs','lua5.4'],text=True))
  subprocess.run(['c++','-std=c++23','-O0',str(path),*flags,'-o',str(binary)],check=True,timeout=45)
  result=subprocess.run([str(binary)],input='\n'.join(code for _,code in CASES)+'\n',text=True,capture_output=True,check=True,timeout=4)
  observations=[json.loads(line) for line in result.stdout.splitlines()];assert len(observations)==len(CASES)
  results.extend(dict(version=version,case=name,source=code,**obs) for (name,code),obs in zip(CASES,observations));print(f'{version}: {len(observations)} actual gesture binding/registry/payload observations')
 content=json.dumps({'format':'hye-gesture-source-fixtures-1','meaning':'Actual binding, descriptors, direction/modifier parsers, add/remove registry, and callback payload functions. Builtin actions, input objects, dispatchers, VM manager, base distance and pointer ownership are fixture adapters. No native input, renderer, VM timeout, userdata ABI or compositor certificate.','cases':results},indent=2,sort_keys=True)+'\n'
 path=ROOT/'data/gesture-source-fixtures.json'
 if args.check:
  if path.read_text()!=content:raise RuntimeError('Gesture observations differ; inspect pinned source/profile')
 else:path.write_text(content)
