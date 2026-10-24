"""Explicit compiler/library selection for trusted reference fixtures only."""
import shlex
import subprocess


def add_arguments(parser):
    parser.add_argument('--lua-package', choices=['lua5.4', 'lua5.5'], default='lua5.4')
    parser.add_argument('--compiler', default='c++')
    parser.add_argument('--cpp-standard', choices=['c++23', 'c++26', 'c++2c'], default='c++23')


def flags(args):
    release = subprocess.check_output(['pkg-config', '--modversion', args.lua_package], text=True).strip()
    if not release.startswith(args.lua_package.removeprefix('lua') + '.'):
        raise RuntimeError('Requested fixture package and reported release disagree')
    return shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', args.lua_package], text=True))


def guard(args):
    family = args.lua_package.removeprefix('lua')
    return '#include <lua.hpp>\nstatic_assert(LUA_VERSION_NUM == ' + family.replace('.', '0') + ');\n'


def require_output(args, canonical):
    alternate = args.lua_package != 'lua5.4' or args.cpp_standard != 'c++23'
    if alternate and (args.output is None or args.output.resolve() == canonical.resolve()):
        raise RuntimeError('An alternate context requires an explicit different output; canonical legacy fixtures cannot be overwritten')
