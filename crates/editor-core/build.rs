fn main() {
    println!("cargo:rerun-if-changed=native/regex.cpp");
    let library = pkg_config::Config::new()
        .probe("re2")
        .expect("System RE2 development package is required");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("native/regex.cpp")
        .flag_if_supported("-std=c++17");
    for path in library.include_paths {
        build.include(path);
    }
    build.compile("hye_regex");
    println!("cargo:rerun-if-changed=native/lua_syntax.cpp");
    let lua = pkg_config::Config::new().probe("lua5.4").expect(
        "Lua 5.4 development headers/library are required for compilation-only source validation",
    );
    let mut syntax = cc::Build::new();
    syntax
        .cpp(true)
        .file("native/lua_syntax.cpp")
        .flag_if_supported("-std=c++17");
    for path in lua.include_paths {
        syntax.include(path);
    }
    syntax.compile("hye_lua_syntax");
}
