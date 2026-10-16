"""Independent ordered-language/loader observations; only trusted private fixture Lua runs."""
import argparse
import concurrent.futures
import copy
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import module_load_probe as loader

paths = loader.paths
ROOT = paths.ROOT
FILES = {
    'value.lua': "emit('value');return {n=2,bump=function(self)self.n=self.n+1;return self.n end}",
    'false.lua': "emit('false');return false",
    'none.lua': "emit('none')",
    'nil.lua': "emit('nil');return nil",
    'ret.lua': "local name,data=...;emit({name=name,data=data});return 7,9",
    'local.lua': "local private=9;return private",
    'counter.lua': "count=(count or 0)+1;emit(count);return count,4",
    'fail.lua': "emit('fail');error('failure',0)",
    'prefix.lua': "emit('prefix');error(\"module 'prefix' not found deliberately\",0)",
    'default_prefix.lua': "emit('default-prefix');error(\"module 'default_prefix' not found deliberately\")",
    'bad.lua': "emit('must-not-run');local value <const> = 1;value=2",
    'file.lua': "emit('file');return ...",
    'a.lua': "emit('a');local b=require('b');emit('a-after');return b",
    'b.lua': "emit('b');return {n=7}",
    'assigned_false.lua': "emit('assigned-false');package.loaded.assigned_false=false",
    'cycle.lua': "depth=(depth or 0)+1;if depth==1 then require('cycle') end;return depth",
    'env.lua': "emit(value);return value",
    'where.lua': "local ok,e=pcall(function()error\n(\n'x'\n)end);emit(e);return true",
}
CASES = [
    ('captured_cells', "local x=1;local function f()return x end;x=2;emit(f())"),
    ('later_local_not_visible', "local function f()return later end;local later=7;emit(f()==nil)"),
    ('block_shadowing_outer_mutation', "local x=1;do local f=function()x=x+1 end;f();local x=9 end;emit(x)"),
    ('iteration_closure_cells', "local f={};for i=1,3 do f[i]=function()return i end end;emit({f[1](),f[2](),f[3]()})"),
    ('assignment_addresses', "local i=1;local t={};i,t[i]=i+1,7;emit({i=i,old=t[1],new_nil=t[2]==nil})"),
    ('multiple_result_adjustment', "local function f()return 1,nil,3 end;local a,b,c=f();emit({a=a,b_nil=b==nil,c=c});emit({(f())});emit(select('#',f()))"),
    ('varargs_and_zero_results', "local function f(a,...)return a,select('#',...),... end;local function g()end;emit({f(1,2,3)});emit(select('#',g()));emit(select('#',(g())))"),
    ('cached_table_mutation', "local a=require('value');a:bump();local b=require('value');emit({same=a==b,n=b.n})"),
    ('false_reloads', "emit(require('false'));emit(require('false'))"),
    ('nil_and_no_return_cache', "emit(require('nil'));emit(require('nil'));emit(require('none'));emit(require('none'))"),
    ('require_loader_data_and_hit_arity', "emit({require('ret')});emit({require('ret')})"),
    ('preload_loader_arguments', "package.preload.x=function(...)emit({n=select('#',...),...});return 9 end;emit({require('x')});emit({require('x')})"),
    ('environment_shadowing', "local log=emit;local _ENV={value=7};log(value)"),
    ('module_local_isolation', "local private=3;emit(require('local'));emit(private)"),
    ('shared_global_writes', "emit(require('counter'));package.loaded.counter=false;emit(require('counter'))"),
    ('preload_registry_survives_global_rebind', "local p=package.preload;p.x=function()return 9 end;package.preload={};emit(require('x'))"),
    ('loaded_global_registry_diverge', "local a=require('value');package.loaded={};emit(a==require('value'))"),
    ('safe_error_cache', "local a=require('fail');local b=require('fail');emit({same=a==b,a=a,b=b})"),
    ('original_error_propagates', "emit(pcall(__require,'fail'));emit(pcall(__require,'fail'))"),
    ('literal_error_prefix_propagates', "emit(pcall(require,'prefix'));emit(pcall(require,'prefix'))"),
    ('default_error_location_prevents_prefix_match', "local a=require('default_prefix');local b=require('default_prefix');emit(a==b)"),
    ('non_table_global_loaded', "package.loaded=false;local a=require('fail');local b=require('fail');emit(a==b)"),
    ('module_semantic_compile_error', "local a=require('bad');local b=require('bad');emit({same=a==b,a=a})"),
    ('original_compile_error_includes_loader_name', "local ok,message=pcall(__require,'bad');emit({ok,message})"),
    ('root_semantic_compile_error', "emit('must-not-run');local value <const> = 1;value=2"),
    ('loadfile_delayed_and_multiple_returns', "local f=loadfile(__fixture_root..'/config/file.lua');emit('before');emit({f(4,5)})"),
    ('loadfile_function_identity', "local f=loadfile(__fixture_root..'/config/file.lua');local g=loadfile(__fixture_root..'/config/file.lua');emit(f==g)"),
    ('dofile_repeats_without_cache', "emit({dofile(__fixture_root..'/config/counter.lua')});emit({dofile(__fixture_root..'/config/counter.lua')})"),
    ('uncalled_function_is_inert', "local function later()return require('never')end;emit('done')"),
    ('nested_module_order_and_identity', "emit('before');local a=require('a');emit('after');emit(a==require('b'))"),
    ('loop_block_return', "local i=0;while i<2 do i=i+1 end;repeat local n=i+1;i=n until n==4;emit(i)"),
    ('integer_wrap_and_floor_modulo', "local m=9223372036854775807;emit({m+1,-5//2,-5%2,5//-2,5%-2})"),
    ('integer_float_comparison_boundary', "local i=9223372036854775807;local f=9223372036854775808.0;emit({less=i<f,greater=f>i,same=i==f})"),
    ('float_integer_key_identity', "local t={};t[1.0]=7;emit({t[1],t[1.0]})"),
    ('nil_module_retains_assigned_false', "emit(require('assigned_false'));emit(require('assigned_false'))"),
    ('finite_recursion_no_loading_sentinel', "emit(require('cycle'))"),
    ('host_call_argument_order', "hl.config({general={border_size=require('b').n}});hl.config({general={border_size=3}})"),
    ('protected_error_c_frame_has_no_location', "local ok,message=pcall(error,'literal');emit({ok,message})"),
    ('exact_integer_float_order', "local i=9007199254740993;emit({i>9007199254740992.0,9007199254740992.0<i,1<1.5,-2< -1.5,-1> -1.5,1.5<2})"),
    ('missing_require_propagates_without_fallback', "emit(pcall(require,'absent'));emit(pcall(__require,'absent'))"),
    ('missing_file_loads', "local f=loadfile(__fixture_root..'/config/absent.lua');emit(f==nil);emit(pcall(dofile,__fixture_root..'/config/absent.lua'))"),
    ('if_elseif_else_order', "if false then emit('never') elseif true then emit('elseif') else emit('never') end;if true then emit('then') else emit('never') end;if false then emit('never') else emit('else') end"),
    ('local_function_whitespace_shadowing', "local f=3;do local\tfunction f()return 9 end;emit(f())end;emit(f)"),
    ('empty_locals_and_method_declaration', "local a,b;local t={n=2};function t:inc(x)self.n=self.n+x;return self.n end;emit(a==nil and b==nil);emit(t:inc(3))"),
    ('loadfile_captured_environment', "local f=loadfile(__fixture_root..'/config/env.lua','t',{emit=emit,value=8});emit({f()})"),
    ('error_location_uses_argument_opening_line', "require('where')"),
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    rows = [r for r in paths.MANIFEST['source_files'] if Path(r['path']).name in ['ConfigManager.cpp', 'MiscFunctions.cpp']]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        sources = dict(pool.map(paths.fetch, rows))
    lua_release = subprocess.check_output(['pkg-config','--modversion','lua5.4'],text=True).strip()
    matrix=json.loads((ROOT/'data/lua-profiles.json').read_text())
    profile=next((p for p in matrix['profiles'] if p['release']==lua_release),None)
    if profile is None and args.output is None:
        raise RuntimeError(f'Unverified Lua release {lua_release}; collect it explicitly before adding a profile')
    observations = []
    # The only host-call adapter is an inert recorder. It does not stand in for field codecs.
    main = loader.MAIN.replace("__trace={};function emit(v)table.insert(__trace,v)end", "__trace={};function emit(v)table.insert(__trace,v)end;hl={config=function(v)table.insert(__trace,{__api='hl.config',value=v})end}")
    main=main.replace('int main(int argc,char**argv){','int main(int argc,char**argv){std::cerr<<"HYE_LUA_RELEASE="<<LUA_RELEASE<<"\\n";')
    signatures = ['static std::string normalizedConfigPath', 'static void trackConfigPath', 'static bool isExplicitRequirePath', 'static bool hasGlobMeta', 'static std::string resolveRequirePath', 'static std::optional<std::string> resolveExplicitLuaRequireFile', 'static void trackWildcardParentDirectory', 'static std::expected<std::vector<std::string>, std::string> expandRequireWildcard', 'static bool pushPackageLoaded', 'static void setPackageLoaded', 'static int requireWildcard', 'static void trackRequiredLuaModulePath', 'static int safeLuaRequire']
    with tempfile.TemporaryDirectory(prefix='hye-ordered-probe-') as folder:
        root = Path(folder)
        (root/'home').mkdir()
        for name, code in FILES.items():
            target = root/'config'/name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(code)
        for version in ['baseline', 'current']:
            manager = sources[version, 'ConfigManager.cpp']
            cpp = loader.HEADER + paths.block(sources[version, 'MiscFunctions.cpp'], 'std::string absolutePath')
            cpp += ''.join(paths.block(manager, signature) for signature in signatures)
            cpp += loader.setup(manager) + loader.reload_cache(manager) + main
            source = root/'probe.cpp'
            source.write_text(cpp)
            flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'lua5.4'], text=True))
            binary = root/'probe'
            subprocess.run(['c++', '-std=c++23', '-O0', str(source), *flags, '-o', str(binary)], check=True, timeout=45)
            result = subprocess.run([str(binary), str(root)], input='\n'.join(code for _, code in CASES)+'\n', text=True, capture_output=True, check=True, timeout=6, cwd=root, env={'HOME':str(root/'home')})
            marker=[line.removeprefix('HYE_LUA_RELEASE=Lua ') for line in result.stderr.splitlines() if line.startswith('HYE_LUA_RELEASE=')]
            if marker != [lua_release]:
                raise RuntimeError(f'Lua header/pkg-config profile mismatch: {marker} versus {lua_release}')
            lines = result.stdout.replace(str(root), '<root>').splitlines()
            if len(lines) != len(CASES):
                raise RuntimeError('Ordered probe observation count differs')
            observations.extend({'version':version, 'case':name, 'source':code, **json.loads(line)} for (name, code), line in zip(CASES, lines))
            print(f'{version}: {len(lines)} ordered language/loader observations')
    content = json.dumps({'format':'hye-ordered-module-fixtures-1', 'meaning':'Real Lua 5.4 and exact pinned require/cache/searcher code, private trusted module fixtures and an inert hl.config argument recorder. No user Lua execution, field-conversion/effective-state, authorization, compositor or ABI certificate.', 'audit_manifest_sha256':hashlib.sha256((ROOT/'data/source-graph-semantics.json').read_bytes()).hexdigest(), 'module_files':FILES, 'cases':observations}, indent=2, sort_keys=True)+'\n'
    target = args.output or ROOT/'data/ordered-module-fixtures.json'
    if args.check:
        expected=json.loads(target.read_text())
        actual=json.loads(content)
        if args.output is None:
            expected=materialize(expected,profile)
        if expected != actual:
            for left, right in zip(expected['cases'], actual['cases']):
                for field in sorted(set(left) | set(right)):
                    if left.get(field) != right.get(field):
                        print(json.dumps({'case':right['case'],'version':right['version'],'field':field,'expected':left.get(field),'actual':right.get(field)}),file=sys.stderr)
            raise RuntimeError(f'Ordered module observations differ for Lua {lua_release}')
    else:
        if args.output is None and lua_release != matrix['canonical_release']:
            raise RuntimeError('Use --output to collect noncanonical profile observations')
        target.write_text(content)


def materialize(base,profile):
    expected=copy.deepcopy(base)
    for change in profile['ordered_overrides']:
        case=next(row for row in expected['cases'] if row['case']==change['case'] and row['version']==change['version'])
        case.update(change['fields'])
    return expected



if __name__ == '__main__':
    main()
