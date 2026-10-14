// Original bounded RE2 adapter. The application invokes untrusted regex work in its restricted child.
#include <re2/re2.h>
#include <algorithm>
#include <cstring>
#include <string>
extern "C" int hye_regex_match(const char* pattern, size_t pattern_len, const char* text, size_t text_len, char* error, size_t error_len) noexcept {
    auto failure = [&](const std::string& message) {
        if (error_len) { const auto n = std::min(error_len - 1, message.size()); std::memcpy(error, message.data(), n); error[n] = 0; }
        return -1;
    };
    try {
        RE2::Options options; options.set_log_errors(false); options.set_max_mem(8 * 1024 * 1024);
        RE2 regex(re2::StringPiece(pattern, pattern_len), options);
        if (!regex.ok()) return failure(regex.error());
        return RE2::FullMatch(re2::StringPiece(text, text_len), regex) ? 1 : 0;
    } catch (...) { return failure("Regex engine could not complete within its allocation policy"); }
}
