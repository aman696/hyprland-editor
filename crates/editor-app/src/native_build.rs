//! Source build requests are metadata, never detected compositor dependencies.
use hyprland_editor_core::{source::display_path_bytes, AUDITED_MAIN, BASELINE};
use std::sync::LazyLock;
static PROFILES: LazyLock<serde_json::Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/native-build-profiles.json"))
        .expect("Bundled source build requests")
});

pub(super) fn note(version: &str, release: &str) -> String {
    let release = display_path_bytes(release.chars().take(40).collect::<String>().as_bytes());
    let key=match version {BASELINE=>"baseline",AUDITED_MAIN=>"current",_=>return format!("Model: Lua {release}. Target build requirements and runtime compatibility are unverified.")};
    let target = PROFILES["snapshots"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["version"] == key));
    let Some(target) = target else {
        return format!("Model: Lua {release}. Target build requirements and runtime compatibility are unverified.");
    };
    let minimum = target["lua_pkgconfig_candidates"]
        .as_array()
        .and_then(|v| {
            v.iter()
                .filter_map(|v| v.as_str())
                .find_map(|v| v.strip_prefix("lua>="))
        })
        .unwrap_or("unresolved");
    let standard = target["requested_cpp_standard"]
        .as_u64()
        .map_or_else(|| "unresolved".into(), |v| v.to_string());
    format!("Model: Lua {release}. Audited source searches Lua {minimum} candidates and requests C++{standard}. Its search can also select other Lua versions. Compatibility with the selected compositor is unverified.")
}
