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
}
