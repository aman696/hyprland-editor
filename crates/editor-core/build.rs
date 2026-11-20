fn main() {
    let native54 = std::env::var_os("CARGO_FEATURE_NATIVE54").is_some();
    let native55 = std::env::var_os("CARGO_FEATURE_NATIVE55").is_some();
    assert!(
        native54 != native55,
        "Select exactly one native Lua family; build each worker image independently"
    );
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
    println!("cargo:rerun-if-changed=../editor-lua55/native/lua55.cpp");
    println!("cargo:rerun-if-changed=../editor-lua55/native/bridge.inc");
    println!("cargo:rerun-if-changed=native/complex_text.cpp");
    let package = if native55 { "lua5.5" } else { "lua5.4" };
    let lua = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe(package)
        .expect("The explicitly selected Lua development headers/library are required");
    if native55 {
        assert!(
            matches!(lua.version.as_str(), "5.5.0" | "5.5.1"),
            "The source worker supports only reviewed Lua 5.5.0/5.5.1 packages"
        );
    } else {
        assert!(
            lua.version.starts_with("5.4."),
            "The legacy image requires Lua 5.4"
        );
    }
    let mut syntax = cc::Build::new();
    syntax
        .cpp(true)
        .file(if native55 {
            "../editor-lua55/native/lua55.cpp"
        } else {
            "native/lua_syntax.cpp"
        })
        .file("native/complex_text.cpp")
        .flag_if_supported("-std=c++17");
    for path in &lua.include_paths {
        syntax.include(path);
    }
    syntax.compile("hye_lua_syntax");
    let linked = pkg_config::Config::new()
        .probe(package)
        .expect("Cannot link the explicitly selected Lua family");
    assert_eq!(
        lua.version, linked.version,
        "Lua package changed during the build"
    );
    assert_eq!(
        lua.include_paths, linked.include_paths,
        "Lua headers changed during the build"
    );
}
