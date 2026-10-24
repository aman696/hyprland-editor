// Legacy image: retain the Lua 5.4 API and two-argument constructor.
#include <lua.hpp>
static_assert(LUA_VERSION_NUM == 504);
#define HYE_LUA_API(name) hye_lua54_##name
static lua_State* hye_newstate(lua_Alloc allocator, void* context) {
    return lua_newstate(allocator, context);
}
#include "../../editor-lua55/native/bridge.inc"
