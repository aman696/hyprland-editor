#!/usr/bin/env python3
"""Narrow source conformance: public Lua named-rule registration and RE2 matching.

Compile exact hash-pinned functions with inert mock rule/descriptor services.
The probe uses real Lua/RE2 but not a compositor, effects parser, window, plugin,
command, IPC socket or userdata implementation. Mocks do not certify those parts.
Downloaded functions are temporary and never linked into the application.
"""
import hashlib
import json
import subprocess
import tempfile
import urllib.request
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
MANIFEST = json.loads((ROOT / 'data/rule-semantics.json').read_text())

def function(source, signature):
    start = source.index(signature)
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]

HEADER = r'''
#include <algorithm>
#include <cassert>
#include <expected>
#include <format>
#include <functional>
#include <iostream>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_set>
#include <utility>
#include <vector>
#include <lua.hpp>
#include <re2/re2.h>
template<class T> using SP=std::shared_ptr<T>;
template<class T> using UP=std::unique_ptr<T>;
template<class T,class... A> auto makeShared(A&&... a){return std::make_shared<T>(std::forward<A>(a)...);}
template<class T,class... A> auto makeUnique(A&&... a){return std::make_unique<T>(std::forward<A>(a)...);}
template<class T,class U> T sc(U u){return static_cast<T>(u);}
namespace Desktop::Rule {
enum eRuleType {RULE_TYPE_WINDOW,RULE_TYPE_LAYER};
struct IRule {
std::string m_name;bool m_enabled=true;std::map<std::string,std::string> m_matchEngines;
IRule(const std::string& n=""):m_name(n){} virtual ~IRule()=default;
virtual eRuleType type()=0;
void registerMatch(const std::string& key,const std::string& value){m_matchEngines[key]=value;}
void setEnabled(bool);bool isEnabled()const;bool canMatch()const;
};
struct Effect {using storageType=int;using valueType=std::string;int key;std::string raw,value;};
}
'''
AFTER_TEMPLATE = r'''
namespace Desktop::Rule {
struct CWindowRule:CRuleWithEffects<Effect,RULE_TYPE_WINDOW> {
CWindowRule(const std::string& name=""):CRuleWithEffects(name){}
std::expected<std::string,std::string> parseEffect(int,const std::string& value)override{return value;}
};
struct CLayerRule:CRuleWithEffects<Effect,RULE_TYPE_LAYER> {
CLayerRule(const std::string& name=""):CRuleWithEffects(name){}
std::expected<std::string,std::string> parseEffect(int,const std::string& value)override{return value;}
};
struct Engine {std::vector<SP<IRule>> registered;void registerRule(SP<IRule> r){registered.push_back(r);}};
Engine* ruleEngine(){static Engine e;return &e;}
struct Effects {std::optional<int> get(std::string_view key){return key=="mock_plugin"?std::optional<int>(9):std::nullopt;}};
Effects* windowEffects(){static Effects e;return &e;} Effects* layerEffects(){return windowEffects();}
std::optional<std::string> matchPropFromString(const std::string& property){return property=="unknown"?std::nullopt:std::optional<std::string>(property);}
class CRegexMatchEngine {
UP<re2::RE2> m_regex;bool m_negative=false;
public:CRegexMatchEngine(const std::string&);bool match(const std::string&);
};
}
constexpr int PARSE_ERROR_OK=0;
struct ParseError {int errorCode=PARSE_ERROR_OK;std::string message;};
struct ILuaConfigValue {std::string value;virtual ~ILuaConfigValue()=default;
ParseError parse(lua_State* L){if(lua_isboolean(L,-1)){value=lua_toboolean(L,-1)?"true":"false";return {};}
if(lua_isstring(L,-1)){value=lua_tostring(L,-1);return {};}
return {1,"mock scalar only"};}
std::string toString(){return value;}};
struct Desc {const char* name;int effect;std::function<ILuaConfigValue*()> factory=[](){return new ILuaConfigValue;};};
const std::vector<Desc> LAYER_RULE_EFFECT_DESCS={{"flag",1}};
struct CConfigManager {
std::map<std::string,SP<Desktop::Rule::CWindowRule>> m_luaWindowRules;
std::map<std::string,SP<Desktop::Rule::CLayerRule>> m_luaLayerRules;
std::vector<std::string> errors;
void addError(const std::string& e){errors.push_back(e);}
};
namespace Internal {
const std::vector<Desc> WINDOW_RULE_EFFECT_DESCS={{"flag",1}};
const Desc* findDescByName(const std::vector<Desc>& all,std::string_view key){for(const auto& d:all)if(d.name==key)return &d;return nullptr;}
std::string getSourceInfo(lua_State*){return "fixture";}
std::expected<std::string,std::string> ruleValueToString(lua_State* L){ILuaConfigValue v;auto e=v.parse(L);if(e.errorCode)return std::unexpected(e.message);return v.value;}
std::expected<void,std::string> addWindowRuleEffectFromLua(lua_State* L,const Desc& d,const SP<Desktop::Rule::CWindowRule>& r){auto v=ruleValueToString(L);if(!v)return std::unexpected(v.error());return r->addEffect(d.effect,*v);}
}
namespace Supplementary {
enum {REFRESH_WINDOW_STATES,REFRESH_RULES};
struct Refresher{void scheduleRefresh(int){}};Refresher* refresher(){static Refresher r;return &r;}
}
namespace Objects {
struct CLuaWindowRule{static void push(lua_State* L,const SP<Desktop::Rule::CWindowRule>&){lua_pushnil(L);}};
struct CLuaLayerRule{static void push(lua_State* L,const SP<Desktop::Rule::CLayerRule>&){lua_pushnil(L);}};
}
using namespace Desktop::Rule;
'''
MAIN = r'''
void registerFunction(lua_State* L,CConfigManager* manager,const char* name,lua_CFunction fn){lua_pushlightuserdata(L,manager);lua_pushcclosure(L,fn,1);lua_setfield(L,-2,name);}
int main(){
auto* L=luaL_newstate();assert(L);CConfigManager m;
lua_newtable(L);registerFunction(L,&m,"window_rule",hlWindowRule);registerFunction(L,&m,"layer_rule",hlLayerRule);lua_setglobal(L,"hl");
const char* source=R"LUA(
hl.window_rule{name='shared',enabled=false,match={class='old',title='kept'},flag=true}
hl.layer_rule{name='shared',match={namespace='panel'},flag=true}
hl.window_rule{match={class='anonymous'},flag=false}
hl.window_rule{name='shared',match={class='new'},flag=false}
hl.window_rule{name='empty'}
hl.window_rule{name=''}
hl.window_rule{name=''}
hl.window_rule{name='integer',match={fullscreen_state_internal=2}}
hl.window_rule{name='fractional',match={fullscreen_state_internal=2.5}}
)LUA";
assert(luaL_loadstring(L,source)==LUA_OK);assert(lua_pcall(L,0,0,0)==LUA_OK);assert(m.errors.empty());
auto r=m.m_luaWindowRules.at("shared");assert(r->isEnabled());assert(r->canMatch());
assert(r->m_matchEngines.at("class")=="new");assert(r->m_matchEngines.at("title")=="kept");
assert(r->effects().size()==2);assert(r->effects()[0].raw=="true");assert(r->effects()[1].raw=="false");
assert(r->effectsSet().size()==1);assert(ruleEngine()->registered.size()==8);
assert(ruleEngine()->registered[0]==r);assert(ruleEngine()->registered[1]==m.m_luaLayerRules.at("shared"));
assert(!m.m_luaWindowRules.at("empty")->canMatch());
assert(m.m_luaWindowRules.at("integer")->m_matchEngines.at("fullscreen_state_internal")=="2");
assert(m.m_luaWindowRules.at("fractional")->m_matchEngines.at("fullscreen_state_internal")=="0");
assert(!CRegexMatchEngine("fire").match("firefox"));assert(CRegexMatchEngine("fire.*").match("firefox"));
assert(!CRegexMatchEngine("negative:fire.*").match("firefox"));assert(CRegexMatchEngine("negative:fire.*").match("other"));
lua_close(L);std::cout<<"named accumulation, enabled reset, retained predicates, registration position, empty matches, Lua integer conversion and RE2 FullMatch passed\n";
}
'''
with tempfile.TemporaryDirectory(prefix='hye-rule-probe-') as temp:
    directory = Path(temp)
    for version in ['baseline', 'current']:
        source = {}
        required = ['src/config/lua/bindings/LuaBindingsConfigRules.cpp', 'src/desktop/rule/Rule.cpp', 'src/desktop/rule/RuleWithEffects.hpp', 'src/desktop/rule/matchEngine/RegexMatchEngine.cpp']
        for row in MANIFEST['source_files']:
            if row['version'] != version or row['path'] not in required:
                continue
            cache = ROOT / '.dev/rule-reference' / f'{version}-{row["path"].replace("/", "_")}'
            data = cache.read_bytes() if cache.is_file() else urllib.request.urlopen(f'https://raw.githubusercontent.com/hyprwm/Hyprland/{row["commit"]}/{row["path"]}', timeout=20).read()
            if hashlib.sha256(data).hexdigest() != row['sha256']:
                raise RuntimeError('Pinned rule source hash mismatch')
            source[Path(row['path']).name] = data.decode()
        template = source['RuleWithEffects.hpp'][source['RuleWithEffects.hpp'].index('namespace Desktop::Rule'):]
        cpp = HEADER + template + AFTER_TEMPLATE
        for signature in ['void IRule::setEnabled', 'bool IRule::isEnabled', 'bool IRule::canMatch']:
            cpp += function(source['Rule.cpp'], signature)
        for signature in ['CRegexMatchEngine::CRegexMatchEngine', 'bool CRegexMatchEngine::match']:
            cpp += function(source['RegexMatchEngine.cpp'], signature)
        for signature in ['static int hlWindowRule', 'static int hlLayerRule']:
            cpp += function(source['LuaBindingsConfigRules.cpp'], signature)
        cpp += MAIN
        path = directory / 'probe.cpp'
        path.write_text(cpp)
        binary = directory / 'probe'
        flags = subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4', 're2'], text=True).split()
        subprocess.run(['c++', '-std=c++23', '-O0', str(path), '-o', str(binary), *flags], check=True, timeout=30)
        result = subprocess.run([str(binary)], check=True, capture_output=True, text=True, timeout=3)
        print(version + ': ' + result.stdout.strip())
