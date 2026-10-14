#!/usr/bin/env python3
"""Run reviewed merge functions from exact pinned source against inert C++ data.

This proves only literal consolidation/default ordering. No Lua, compositor,
selector matching implementation or runtime workspace is started.
"""
import hashlib,json,subprocess,tempfile,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
manifest=json.loads((ROOT/'data/workspace-semantics.json').read_text())
def function(source,signature):
    start=source.index(signature);body=source.index('{',start);depth=1;end=body+1
    while depth:
        depth += (source[end]=='{')-(source[end]=='}');end+=1
    return source[start:end]
header='''#include <algorithm>
#include <cassert>
#include <iostream>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>
template<class T> using SP=std::shared_ptr<T>;
template<class T,class... A> auto makeShared(A&&... a){return std::make_shared<T>(std::forward<A>(a)...);}
namespace Monitor {struct IMonitorIdentifiable {std::string name="DP-1";bool matchesStaticSelector(std::string_view s)const{return name==s;}};using IMonitorAddressable=IMonitorIdentifiable;}
constexpr long WORKSPACE_INVALID=-1;
namespace Config {
struct CWorkspaceRule {
std::string m_monitor,m_workspaceString,m_workspaceName;
long m_workspaceId=WORKSPACE_INVALID;
std::optional<bool> m_isDefault,m_isPersistent,m_noBorder,m_noRounding,m_decorate,m_noShadow;
std::optional<int> m_gapsIn,m_gapsOut,m_floatGaps,m_borderSize;
std::optional<std::string> m_onCreatedEmptyRunCmd,m_defaultName,m_layout,m_animationStyle;
std::map<std::string,std::string> m_layoutopts;
bool enabled=true;
void mergeLeft(const CWorkspaceRule&);
bool isEnabled()const{return enabled;}
};
struct CWorkspaceRuleManager {
std::vector<SP<CWorkspaceRule>> m_rules;
SP<CWorkspaceRule> add(CWorkspaceRule&&);
SP<CWorkspaceRule> replaceOrAdd(CWorkspaceRule&&);
std::string getDefaultWorkspaceFor(const Monitor::IMonitorIdentifiable&);
};
}
using namespace Config;
'''
main='''int main(){
CWorkspaceRuleManager m;
CWorkspaceRule a;a.m_workspaceString="1";a.m_monitor="DP-1";a.m_isPersistent=true;a.m_floatGaps=5;a.m_layoutopts["a"]="true";
a.m_borderSize=7;a.m_onCreatedEmptyRunCmd="never execute";
auto first=m.replaceOrAdd(std::move(a));
CWorkspaceRule b;b.m_workspaceString="1";b.enabled=false;b.m_monitor="";b.m_isPersistent=false;b.m_floatGaps=0;b.m_layoutopts["b"]="false";
b.m_borderSize=0;b.m_onCreatedEmptyRunCmd="";
auto second=m.replaceOrAdd(std::move(b));
assert(m.m_rules.size()==1);assert(first==second);assert(first->isEnabled());
assert(first->m_monitor=="DP-1");assert(first->m_isPersistent==false);assert(first->m_floatGaps==0);assert(first->m_borderSize==0);
assert(first->m_onCreatedEmptyRunCmd=="");assert(first->m_layoutopts.size()==2);
CWorkspaceRule disabled;disabled.m_workspaceString="2";disabled.enabled=false;m.replaceOrAdd(std::move(disabled));
CWorkspaceRule later;later.m_workspaceString="2";m.replaceOrAdd(std::move(later));assert(m.m_rules.size()==3);
CWorkspaceRuleManager defaults;
CWorkspaceRule off;off.m_workspaceString="disabled";off.enabled=false;off.m_isDefault=true;off.m_monitor="DP-1";defaults.add(std::move(off));
CWorkspaceRule one;one.m_workspaceString="first";one.m_isDefault=true;one.m_monitor="DP-1";defaults.add(std::move(one));
CWorkspaceRule two;two.m_workspaceString="second";two.m_isDefault=true;two.m_monitor="DP-1";defaults.add(std::move(two));
assert(defaults.getDefaultWorkspaceFor(Monitor::IMonitorIdentifiable{})=="first");
std::cout<<"default order, literal merge, enabled retention, empty monitor, zero/false/empty overrides and option union passed\\n";
}
'''
with tempfile.TemporaryDirectory(prefix='hye-workspace-probe-') as temp:
    directory=Path(temp)
    for version in ['baseline','current']:
        source={}
        for row in manifest['source_files']:
            if row['version']!=version or row['path'] not in ['src/config/shared/workspace/WorkspaceRule.cpp','src/config/shared/workspace/WorkspaceRuleManager.cpp']:continue
            cache=ROOT/'.dev/workspace-reference'/f'{version}-{row["path"].replace("/","_")}'
            data=cache.read_bytes() if cache.is_file() else urllib.request.urlopen(f'https://raw.githubusercontent.com/hyprwm/Hyprland/{row["commit"]}/{row["path"]}',timeout=20).read()
            if hashlib.sha256(data).hexdigest()!=row['sha256']:raise RuntimeError('Pinned source hash mismatch')
            source[Path(row['path']).name]=data.decode()
        cpp=header+function(source['WorkspaceRule.cpp'],'void CWorkspaceRule::mergeLeft')+function(source['WorkspaceRuleManager.cpp'],'SP<CWorkspaceRule> CWorkspaceRuleManager::add')+function(source['WorkspaceRuleManager.cpp'],'SP<CWorkspaceRule> CWorkspaceRuleManager::replaceOrAdd')+function(source['WorkspaceRuleManager.cpp'],'std::string CWorkspaceRuleManager::getDefaultWorkspaceFor')+main
        path=directory/'probe.cpp';path.write_text(cpp);binary=directory/'probe'
        subprocess.run(['c++','-std=c++23','-O0',str(path),'-o',str(binary)],check=True,timeout=30)
        result=subprocess.run([str(binary)],check=True,capture_output=True,text=True,timeout=3)
        print(version+': '+result.stdout.strip())
