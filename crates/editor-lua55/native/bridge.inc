// Shared bounded adapter. Each including image asserts its own Lua family.
#ifndef HYE_LUA_API
#error "A profile-specific API prefix is required"
#endif
// Text compilation and primitive C-API conversion. No libraries or Lua code run.
#include <cstdlib>
#include <cstdint>
#include <clocale>
#include <cstring>
#include <limits>
#include <new>
extern "C" unsigned int HYE_LUA_API(release)() {
    return LUA_VERSION_RELEASE_NUM;
}
extern "C" int HYE_LUA_API(runtime_matches_header)() {
    return lua_version(nullptr) == LUA_VERSION_NUM;
}
extern "C" size_t HYE_LUA_API(identity)(unsigned char* output, size_t capacity) {
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
struct ProbeOutput {
    unsigned char* bytes;
    size_t capacity;
    size_t written;
    bool exceeded;
};
static int write_probe(lua_State*, const void* data, size_t count, void* context) {
    auto* output = static_cast<ProbeOutput*>(context);
    if (count > output->capacity - output->written) {
        output->exceeded = true;
        return 1;
    }
    if (count)
        std::memcpy(output->bytes + output->written, data, count);
    output->written += count;
    return 0;
}
// Observe the linked compiler, including backports, without invoking Lua code.
// No source/name comes from a request. Only stripped bytes of this fixed probe
// are returned; callers hash them and match an independently checked profile.
extern "C" int HYE_LUA_API(subtract_probe)(unsigned char* output, size_t capacity, size_t* written) {
    if (!output || !written || capacity > 4096)
        return 3;
    *written = 0;
    Budget budget{0};
    lua_State* state = hye_newstate(allocate, &budget);
    if (!state)
        return 2;
    static constexpr const char source[] = "return function(x) return x - 0 end";
    const int loaded = luaL_loadbufferx(state, source, sizeof(source)-1, "@hye-numeric-profile", "t");
    if (loaded != LUA_OK) {
        lua_close(state);
        return loaded == LUA_ERRMEM ? 2 : 1;
    }
    ProbeOutput result{output, capacity, 0, false};
    const int dumped = lua_dump(state, write_probe, &result, 1);
    lua_close(state);
    if (dumped || result.exceeded)
        return 3;
    *written = result.written;
    return 0;
}
extern "C" int HYE_LUA_API(compile)(const unsigned char* source, size_t length,
                                const char* name, unsigned char* error,
                                size_t capacity, size_t* written) {
    Budget budget{0};
    *written = 0;
    lua_State* state = hye_newstate(allocate, &budget);
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

struct Coercions {
    Budget budget{0};
    lua_State* state = nullptr;
};
extern "C" int HYE_LUA_API(numeric_locale_c)() {
    const char* locale = std::setlocale(LC_NUMERIC, nullptr);
    return locale && std::strcmp(locale, "C") == 0;
}
struct Converted {
    unsigned int flags;
    int64_t integer;
    double number;
    size_t length;
    int64_t coerced_integer;
};
static_assert(sizeof(unsigned int) == 4);
struct Conversion {
    int kind;
    int64_t integer;
    double number;
    const unsigned char* bytes;
    size_t length;
    Converted* result;
    unsigned char* text;
    size_t capacity;
    bool exceeded = false;
};
// Only this editor-owned C function is invoked. Allocation failures stay inside
// lua_pcall; no C++ object with a destructor crosses Lua's longjmp boundary.
static int convert_primitive(lua_State* state) {
    auto* input = static_cast<Conversion*>(lua_touserdata(state, 1));
    switch (input->kind) {
        case 1: lua_pushboolean(state, input->integer != 0); break;
        case 2: lua_pushinteger(state, input->integer); break;
        case 3: lua_pushnumber(state, input->number); break;
        case 4: lua_pushlstring(state, reinterpret_cast<const char*>(input->bytes), input->length); break;
        default: lua_pushnil(state); break;
    }
    auto* out = input->result;
    // Check number/integer identity before lua_tolstring mutates numeric slots.
    if (lua_isinteger(state, -1)) {
        out->flags |= 1;
        out->integer = lua_tointeger(state, -1);
    }
    if (lua_isnumber(state, -1)) {
        out->flags |= 2;
        out->number = lua_tonumber(state, -1);
        out->coerced_integer = lua_tointeger(state, -1);
    }
    if (lua_isboolean(state, -1)) {
        out->flags |= 8;
        out->integer = lua_toboolean(state, -1);
    }
    if (lua_isstring(state, -1)) {
        out->flags |= 4;
        size_t length = 0;
        const char* bytes = lua_tolstring(state, -1, &length);
        const void* nul = std::memchr(bytes, 0, length);
        if (nul)
            length = static_cast<const char*>(nul) - bytes;
        if (length > input->capacity)
            input->exceeded = true;
        else {
            std::memcpy(input->text, bytes, length);
            out->length = length;
        }
    }
    return 0;
}
extern "C" void* HYE_LUA_API(coercions_new)() {
    auto* context = new (std::nothrow) Coercions;
    if (!context)
        return nullptr;
    context->state = hye_newstate(allocate, &context->budget);
    if (!context->state) {
        delete context;
        return nullptr;
    }
    return context;
}
extern "C" void HYE_LUA_API(coercions_close)(void* pointer) {
    auto* context = static_cast<Coercions*>(pointer);
    if (context) {
        lua_close(context->state);
        delete context;
    }
}
extern "C" int HYE_LUA_API(convert)(void* pointer, int kind, int64_t integer, double number,
                                const unsigned char* bytes, size_t length, Converted* result,
                                unsigned char* text, size_t capacity) {
    if (!pointer || !result || !text || length > 65536 || kind < 0 || kind > 4 || (length && !bytes))
        return 3;
    auto* context = static_cast<Coercions*>(pointer);
    *result = {};
    Conversion input{kind, integer, number, bytes, length, result, text, capacity};
    // A new state has room for these two nonallocating pushes. Each call resets
    // the stack. All allocating string/conversion operations are protected.
    lua_pushcfunction(context->state, convert_primitive);
    lua_pushlightuserdata(context->state, &input);
    const int status = lua_pcall(context->state, 1, 0, 0);
    lua_settop(context->state, 0);
    return status == LUA_OK && !input.exceeded ? 0 : 2;
}
