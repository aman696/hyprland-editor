// Separate image only: never link this family into the GUI/legacy image.
#include <lua.hpp>
#include <cerrno>
#include <sys/random.h>
static_assert(LUA_VERSION_NUM == 505);
#define HYE_LUA_API(name) hye_lua55_##name
extern "C" int HYE_LUA_API(runtime_matches_header)();
static thread_local bool hye_seed_failed = false;
extern "C" int hye_lua55_seed_failed() {
    return hye_seed_failed;
}
static lua_State* hye_newstate(lua_Alloc allocator, void* context) {
    // No request, environment or saved compositor state supplies this seed.
    // Refuse unavailable entropy rather than falling back to a fixed value.
    hye_seed_failed = false;
    if (!HYE_LUA_API(runtime_matches_header)())
        return nullptr;
    unsigned int seed = 0;
    size_t offset = 0;
    unsigned int attempts = 0;
    while (offset < sizeof(seed) && ++attempts <= 8) {
        const auto count = getrandom(reinterpret_cast<unsigned char*>(&seed) + offset, sizeof(seed) - offset, GRND_NONBLOCK);
        if (count > 0)
            offset += count;
        else if (count < 0 && errno == EINTR)
            continue;
        else
            break;
    }
    if (offset != sizeof(seed)) {
        hye_seed_failed = true;
        return nullptr;
    }
    return lua_newstate(allocator, context, seed);
}
#include "bridge.inc"
