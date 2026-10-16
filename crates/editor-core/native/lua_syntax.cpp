// Compilation only: no libraries are opened and no bytecode/function is invoked.
#include <lua.hpp>
#include <cstdlib>
#include <cstring>
#include <limits>
static_assert(LUA_VERSION_NUM == 504);
extern "C" unsigned int hye_lua54_release() {
    return LUA_VERSION_RELEASE_NUM;
}
extern "C" size_t hye_lua54_identity(unsigned char* output, size_t capacity) {
    const size_t length = strnlen(lua_ident, capacity);
    if (length == capacity)
        return 0;
    std::memcpy(output, lua_ident, length);
    return length;
}
static_assert(sizeof(lua_Integer) == 8 && sizeof(lua_Number) == 8);
static_assert(std::numeric_limits<lua_Integer>::digits == 63);
static_assert(std::numeric_limits<lua_Number>::is_iec559 &&
              std::numeric_limits<lua_Number>::digits == 53);
struct Budget {
    size_t bytes;
};
static void* allocate(void* data, void* pointer, size_t old_size, size_t size) {
    auto* budget = static_cast<Budget*>(data);
    // For new objects Lua's osize is a type tag, not an allocation size.
    if (!pointer)
        old_size = 0;
    if (!size) {
        std::free(pointer);
        budget->bytes -= old_size;
        return nullptr;
    }
    constexpr size_t limit = 8 * 1024 * 1024;
    if (old_size > budget->bytes || size > limit || budget->bytes - old_size > limit - size)
        return nullptr;
    void* result = std::realloc(pointer, size);
    if (result)
        budget->bytes = budget->bytes - old_size + size;
    return result;
}
extern "C" int hye_lua54_compile(const unsigned char* source, size_t length,
                                const char* name, unsigned char* error,
                                size_t capacity, size_t* written) {
    Budget budget{0};
    *written = 0;
    lua_State* state = lua_newstate(allocate, &budget);
    if (!state)
        return 2;
    const int status = luaL_loadbufferx(state, reinterpret_cast<const char*>(source), length, name, "t");
    if (status != LUA_OK && status != LUA_ERRMEM) {
        size_t count = 0;
        const char* message = lua_tolstring(state, -1, &count);
        if (message) {
            if (count > capacity) {
                lua_close(state);
                return 3; // Diagnostic budget: never return a truncated error as exact.
            }
            std::memcpy(error, message, count);
            *written = count;
        }
    }
    lua_close(state);
    return status == LUA_OK ? 0 : status == LUA_ERRMEM ? 2 : 1;
}
