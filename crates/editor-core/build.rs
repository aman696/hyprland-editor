fn main() {
    println!("cargo:rerun-if-changed=native/regex.cpp");
    let library = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("re2")
        .expect("System RE2 development package is required");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("native/regex.cpp")
        .flag_if_supported("-std=c++17");
    for path in &library.include_paths {
        build.include(path);
    }
    build.compile("hye_regex");
    let linked = pkg_config::Config::new()
        .probe("re2")
        .expect("Cannot link RE2");
    assert_eq!(
        library.version, linked.version,
        "RE2 package changed during the build"
    );
    assert_eq!(
        library.include_paths, linked.include_paths,
        "RE2 headers changed during the build"
    );
    println!("cargo:rerun-if-changed=native/lua_syntax.cpp");
    println!("cargo:rerun-if-changed=../editor-lua55/native/bridge.inc");
    println!("cargo:rerun-if-changed=native/complex_text.cpp");
    let lua = pkg_config::Config::new().cargo_metadata(false).probe("lua5.4").expect(
        "Lua 5.4 development headers/library are required for bounded source compilation and primitive conversion",
    );
    let mut syntax = cc::Build::new();
    syntax
        .cpp(true)
        .file("native/lua_syntax.cpp")
        .file("native/complex_text.cpp")
        .flag_if_supported("-std=c++17");
    for path in &lua.include_paths {
        syntax.include(path);
    }
    syntax.compile("hye_lua_syntax");
    let linked = pkg_config::Config::new()
        .probe("lua5.4")
        .expect("Cannot link Lua 5.4");
    assert_eq!(
        lua.version, linked.version,
        "Lua package changed during the build"
    );
    assert_eq!(
        lua.include_paths, linked.include_paths,
        "Lua headers changed during the build"
    );
}
