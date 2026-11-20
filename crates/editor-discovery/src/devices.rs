//! Device names are observations, not a complete effective input configuration.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCategory {
    Pointer,
    Keyboard,
    Tablet,
    Touch,
    Switch,
}

#[derive(Clone, Debug, Serialize)]
pub struct Device {
    pub category: DeviceCategory,
    pub name: Option<String>,
    pub role: Option<String>,
    pub owner_name: Option<String>,
    pub layout: Option<String>,
    pub active_keymap: Option<String>,
}
impl Device {
    /// Pads/tools and switch names are not inferred to be override targets.
    pub fn configurable_name(&self) -> Option<&str> {
        if self.category == DeviceCategory::Switch || self.role.is_some() {
            return None;
        }
        self.name.as_deref().filter(|name| !name.is_empty())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DeviceSnapshot {
    pub identity: SessionIdentity,
    pub commit: String,
    pub version: Option<String>,
    pub sampled_unix_ms: u128,
    pub duration_ms: u128,
    pub devices: Vec<Device>,
    pub unreported_categories: Vec<DeviceCategory>,
}

#[derive(Deserialize)]
struct Owner {
    name: Option<String>,
}
#[derive(Deserialize)]
struct RawDevice {
    name: Option<String>,
    #[serde(rename = "type")]
    role: Option<String>,
    #[serde(rename = "belongsTo")]
    owner: Option<Owner>,
    layout: Option<String>,
    active_keymap: Option<String>,
}
#[derive(Deserialize)]
struct Reply {
    mice: Option<Vec<RawDevice>>,
    keyboards: Option<Vec<RawDevice>>,
    tablets: Option<Vec<RawDevice>>,
    touch: Option<Vec<RawDevice>>,
    switches: Option<Vec<RawDevice>>,
}
fn parse(bytes: &[u8]) -> Result<(Vec<Device>, Vec<DeviceCategory>), String> {
    if bytes.len() > MAX_REPLY {
        return Err("Device response exceeds 2 MiB".into());
    }
    let reply: Reply =
        serde_json::from_slice(bytes).map_err(|_| "Compositor returned malformed device JSON")?;
    let mut devices = vec![];
    let mut missing = vec![];
    for (category, records) in [
        (DeviceCategory::Pointer, reply.mice),
        (DeviceCategory::Keyboard, reply.keyboards),
        (DeviceCategory::Tablet, reply.tablets),
        (DeviceCategory::Touch, reply.touch),
        (DeviceCategory::Switch, reply.switches),
    ] {
        let Some(records) = records else {
            missing.push(category);
            continue;
        };
        if records.len() > 512 - devices.len() {
            return Err("More than 512 device observations were returned".into());
        }
        for raw in records {
            let owner_name = raw.owner.and_then(|owner| owner.name);
            for value in [
                &raw.name,
                &raw.role,
                &owner_name,
                &raw.layout,
                &raw.active_keymap,
            ] {
                if value.as_ref().is_some_and(|value| value.len() > 4096) {
                    return Err("Oversized device observation string".into());
                }
            }
            if raw
                .name
                .as_ref()
                .is_some_and(|name| name.chars().any(char::is_control))
            {
                return Err("Device name contains control characters".into());
            }
            devices.push(Device {
                category,
                name: raw.name,
                role: raw.role,
                owner_name,
                layout: raw.layout,
                active_keymap: raw.active_keymap,
            });
        }
    }
    Ok((devices, missing))
}

pub fn device_snapshot(runtime: &Path, signature: &str) -> Result<DeviceSnapshot, String> {
    let start = Instant::now();
    let session = ReadSession::attach(runtime, signature)?;
    let version: VersionReply = serde_json::from_slice(&session.request(Query::Version)?)
        .map_err(|_| "Compositor returned malformed version JSON")?;
    let pack = match version.commit.as_str() {
        BASE_COMMIT => Some(BASELINE.into()),
        MAIN_COMMIT => Some(AUDITED_MAIN.into()),
        _ => None,
    };
    let (devices, unreported_categories) = parse(&session.request(Query::Devices)?)?;
    session.recheck()?;
    Ok(DeviceSnapshot {
        identity: session.identity,
        commit: version.commit,
        version: pack,
        sampled_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis(),
        duration_ms: start.elapsed().as_millis(),
        devices,
        unreported_categories,
    })
}
pub fn current_session_devices() -> Result<DeviceSnapshot, String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("No user runtime directory; start the editor inside your Hyprland session")?;
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|_| "No selected Hyprland session in this environment")?;
    device_snapshot(&runtime, &signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_snapshot_uses_fixed_reads_and_never_invents_pad_tool_or_switch_targets() {
        let (root, listener) = crate::tests::fixture();
        let server = std::thread::spawn(move || {
            for expected in [b"j/version".as_slice(), b"j/devices"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0; 64];
                let count = stream.read(&mut bytes).unwrap();
                assert_eq!(&bytes[..count], expected);
                let reply = if expected == b"j/version" {
                    br#"{"commit":"unmapped-build"}"#.as_slice()
                } else {
                    br#"{"mice":[{"name":"same-name"}],"keyboards":[{"name":"same-name","layout":"us,de","active_keymap":"English (US)"}],"tablets":[{"type":"tabletPad","belongsTo":{"name":"parent-tablet"}},{"type":"tabletTool"},{"name":"parent-tablet"}],"switches":[{"name":"Lid Switch"}]}"#.as_slice()
                };
                stream.write_all(reply).unwrap();
            }
        });
        let snapshot = device_snapshot(&root, "test").unwrap();
        server.join().unwrap();
        assert!(snapshot.version.is_none());
        assert_eq!(snapshot.unreported_categories, vec![DeviceCategory::Touch]);
        assert_eq!(snapshot.devices.len(), 6);
        assert_eq!(snapshot.devices[0].configurable_name(), Some("same-name"));
        assert_eq!(snapshot.devices[1].layout.as_deref(), Some("us,de"));
        assert!(snapshot.devices[2].configurable_name().is_none());
        assert_eq!(
            snapshot.devices[2].owner_name.as_deref(),
            Some("parent-tablet")
        );
        assert!(snapshot.devices[3].configurable_name().is_none());
        assert_eq!(
            snapshot.devices[4].configurable_name(),
            Some("parent-tablet")
        );
        assert!(snapshot.devices[5].configurable_name().is_none());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn device_payload_bounds_and_unknown_values_do_not_become_guessed_names() {
        for bytes in [
            b"[]".as_slice(),
            br#"{"mice":[],"mice":[]}"#,
            br#"{"keyboards":[{"name":"a","name":"b"}]}"#,
            br#"{"mice":[{"name":"a\nb"}]}"#,
            br#"{"keyboards":[{"active_layout_index":none}]}"#,
        ] {
            assert!(parse(bytes).is_err());
        }
        let too_many = serde_json::to_vec(
            &serde_json::json!({"mice":vec![serde_json::json!({"name":"a"});513]}),
        )
        .unwrap();
        assert!(parse(&too_many).is_err());
        let oversized =
            serde_json::to_vec(&serde_json::json!({"touch":[{"name":"x".repeat(4097)}]})).unwrap();
        assert!(parse(&oversized).is_err());
        assert!(parse(&vec![b' '; MAX_REPLY + 1]).is_err());
        let (devices, missing) =
            parse(br#"{"keyboards":[{}],"unknownCategory":[{"name":"ignored"}]}"#).unwrap();
        assert_eq!(devices.len(), 1);
        assert!(devices[0].configurable_name().is_none());
        assert_eq!(missing.len(), 4);
    }
}
