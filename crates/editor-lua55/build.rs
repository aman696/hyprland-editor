fn main() {
    println!("cargo:rerun-if-changed=native/lua55.cpp");
    println!("cargo:rerun-if-changed=native/bridge.inc");
    if std::env::var_os("CARGO_FEATURE_NATIVE55").is_none() {
        return;
    }
    let lua = pkg_config::Config::new()
        .cargo_metadata(false)
        .range_version("5.5".."5.6")
        .probe("lua5.5")
        .expect("The separate native55 worker requires Lua 5.5 headers/library");
    assert!(
        matches!(lua.version.as_str(), "5.5.0" | "5.5.1"),
        "The compiler worker supports only the explicitly reviewed Lua 5.5.0/5.5.1 packages"
    );
    println!("cargo:rustc-env=HYE_LUA55_PACKAGE_RELEASE={}", lua.version);
    let mut build = cc::Build::new();
    build.cpp(true).file("native/lua55.cpp").std("c++17");
    for path in &lua.include_paths {
        build.include(path);
    }
    build.compile("hye_lua55_bridge");
    // GNU ld resolves static archives from left to right. Emit Lua only after
    // its caller (the adapter), rather than relying on lld's archive rescans.
    let linked = pkg_config::Config::new()
        .range_version("5.5".."5.6")
        .probe("lua5.5")
        .expect("Cannot link the separate Lua 5.5 compiler library");
    assert_eq!(
        lua.version, linked.version,
        "Lua package changed during the build"
    );
    assert_eq!(
        lua.include_paths, linked.include_paths,
        "Lua headers changed during the build"
    );
}
