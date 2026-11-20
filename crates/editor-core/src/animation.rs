//! Non-executing animation declarations, named curve registry and inherited tree resolution.
use crate::{quote_lua, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_OPERATIONS: usize = 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeInfo {
    pub version: String,
    pub name: String,
    pub parent: Option<String>,
    pub internal: bool,
    pub initial: Option<Effective>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effective {
    pub enabled: bool,
    pub speed: f64,
    pub curve: String,
    pub style: String,
}
impl Effective {
    pub fn bezier_duration_ms(&self) -> Option<f64> {
        (!self.curve.starts_with("spring:") || self.curve.len() == 7).then_some(self.speed * 100.)
    }
}
pub fn catalogue(version: &str) -> Vec<NodeInfo> {
    static ALL: std::sync::OnceLock<Vec<NodeInfo>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/animation-semantics.json"))
                .expect("checked animation metadata");
        serde_json::from_value(value["nodes"].clone()).expect("checked animation nodes")
    })
    .iter()
    .filter(|n| n.version == version)
    .cloned()
    .collect()
}
pub fn schema_hash() -> String {
    crate::lua::source_hash(include_str!("../../../data/animation-semantics.json"))
}
fn literal_lua(value: &Value) -> String {
    let raw = value.lua();
    if matches!(value, Value::Float(_)) && !raw.contains(['.', 'e', 'E']) {
        format!("{raw}.0")
    } else {
        raw
    }
}
fn text(s: &str) -> Result<(), String> {
    if s.len() > 16384 || s.contains('\0') {
        Err("Animation text exceeds 16 KiB or contains NUL".into())
    } else {
        Ok(())
    }
}
pub fn number(value: &Value, allow_bool: bool) -> Result<f64, String> {
    let n = match value {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        Value::Bool(b) if allow_bool => f64::from(u8::from(*b)),
        Value::String(s) => {
            text(s)?;
            let s = s.trim_matches(|c: char| c.is_ascii_whitespace());
            if !s.chars().all(|c| c.is_ascii_digit() || ".+-eE".contains(c)) {
                return Err("Numeric string spelling is unresolved".into());
            }
            s.parse::<f64>().map_err(|_| "Expected a literal number")?
        }
        _ => return Err("Expected a literal number".into()),
    };
    if !n.is_finite() || !(n as f32).is_finite() {
        return Err("Animation number exceeds the finite float32 domain".into());
    }
    Ok(n)
}
pub fn enabled(value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(b) => Ok(*b),
        v => match number(v, false)? {
            0. => Ok(false),
            1. => Ok(true),
            _ => Err("Animation enabled needs bool or 0/1".into()),
        },
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields, rename_all = "snake_case")]
pub enum CurveKind {
    Bezier {
        points: [[Value; 2]; 2],
    },
    Spring {
        stiffness: Value,
        damping: Value,
        mass: Value,
        #[serde(default)]
        dampening_alias: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub name: String,
    pub shape: CurveKind,
}
impl Curve {
    pub fn bezier(name: &str) -> Self {
        Self {
            name: name.into(),
            shape: CurveKind::Bezier {
                points: [
                    [Value::Float(0.25), Value::Float(0.1)],
                    [Value::Float(0.25), Value::Float(1.)],
                ],
            },
        }
    }
    pub fn spring(name: &str) -> Self {
        Self {
            name: name.into(),
            shape: CurveKind::Spring {
                stiffness: Value::Float(250.),
                damping: Value::Float(25.),
                mass: Value::Float(1.),
                dampening_alias: true,
            },
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        text(&self.name)?;
        match &self.shape {
            CurveKind::Bezier { points } => {
                for point in points {
                    for v in point {
                        let n = number(v, true)?;
                        if !(-1. ..=2.).contains(&n) {
                            return Err("Bézier coordinates must lie in -1..2".into());
                        }
                    }
                }
            }
            CurveKind::Spring {
                stiffness,
                damping,
                mass,
                ..
            } => {
                for v in [stiffness, damping, mass] {
                    if number(v, false)? as f32 <= 0.5 {
                        return Err(
                            "Spring stiffness, damping and mass must be strictly greater than 0.5"
                                .into(),
                        );
                    }
                }
            }
        }
        Ok(())
    }
    pub fn export_lua(&self) -> Result<String, String> {
        self.validate()?;
        let fields = match &self.shape {
            CurveKind::Bezier {
                points: [[x1, y1], [x2, y2]],
            } => format!(
                "type = \"bezier\", points = {{ {{ {}, {} }}, {{ {}, {} }} }}",
                literal_lua(x1),
                literal_lua(y1),
                literal_lua(x2),
                literal_lua(y2)
            ),
            CurveKind::Spring {
                stiffness,
                damping,
                mass,
                dampening_alias,
            } => format!(
                "type = \"spring\", stiffness = {}, {} = {}, mass = {}",
                literal_lua(stiffness),
                if *dampening_alias {
                    "dampening"
                } else {
                    "damping"
                },
                literal_lua(damping),
                literal_lua(mass)
            ),
        };
        Ok(format!(
            "hl.curve({}, {{ {fields} }})\n",
            quote_lua(&self.name)
        ))
    }
    /// Control-point diagram, not Hyprutils' baked inverse-x interpolation or compositor preview.
    pub fn diagram_point(&self, t: f64) -> Option<[f64; 2]> {
        let CurveKind::Bezier { points } = &self.shape else {
            return None;
        };
        let p1 = [
            number(&points[0][0], true).ok()?,
            number(&points[0][1], true).ok()?,
        ];
        let p2 = [
            number(&points[1][0], true).ok()?,
            number(&points[1][1], true).ok()?,
        ];
        let u = 1. - t;
        Some([
            3. * u * u * t * p1[0] + 3. * u * t * t * p2[0] + t * t * t,
            3. * u * u * t * p1[1] + 3. * u * t * t * p2[1] + t * t * t,
        ])
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub leaf: String,
    pub enabled: Value,
    pub speed: Option<Value>,
    pub bezier: Option<String>,
    pub spring: Option<String>,
    pub style: Option<String>,
}
impl Node {
    pub fn new(leaf: &str) -> Self {
        Self {
            leaf: leaf.into(),
            enabled: Value::Bool(true),
            speed: Some(Value::Float(8.)),
            bezier: Some("default".into()),
            spring: None,
            style: None,
        }
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if !catalogue(version).iter().any(|n| n.name == self.leaf) {
            return Err("Animation node is outside the pinned source tree".into());
        }
        text(&self.leaf)?;
        for name in [&self.bezier, &self.spring, &self.style]
            .into_iter()
            .flatten()
        {
            text(name)?;
        }
        if let Some(speed) = &self.speed {
            number(speed, true)?;
        }
        if enabled(&self.enabled)? {
            let speed = number(
                self.speed
                    .as_ref()
                    .ok_or("Enabled animation requires speed")?,
                true,
            )?;
            if speed <= 0. || speed > 100. || speed as f32 <= 0. {
                return Err("Enabled animation speed must be >0 and <=100".into());
            }
            if self.bezier.is_none() && self.spring.is_none() {
                return Err("Choose a Bézier or spring reference".into());
            }
            if let Some(style) = &self.style {
                validate_style(&self.leaf, style)?;
            }
        }
        Ok(())
    }
    pub fn effective(&self) -> Result<Effective, String> {
        if !enabled(&self.enabled)? {
            return Ok(Effective {
                enabled: false,
                speed: 1.,
                curve: "default".into(),
                style: String::new(),
            });
        }
        Ok(Effective {
            enabled: true,
            speed: f64::from(number(self.speed.as_ref().ok_or("Missing speed")?, true)? as f32),
            curve: if let Some(bezier) = &self.bezier {
                bezier.clone()
            } else {
                format!("spring:{}", self.spring.as_ref().ok_or("Missing curve")?)
            },
            style: self.style.clone().unwrap_or_default(),
        })
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut fields = vec![
            format!("leaf = {}", quote_lua(&self.leaf)),
            format!("enabled = {}", literal_lua(&self.enabled)),
        ];
        if let Some(speed) = &self.speed {
            fields.push(format!("speed = {}", literal_lua(speed)));
        }
        for (key, value) in [
            ("bezier", &self.bezier),
            ("spring", &self.spring),
            ("style", &self.style),
        ] {
            if let Some(value) = value {
                fields.push(format!("{key} = {}", quote_lua(value)));
            }
        }
        Ok(format!("hl.animation({{ {} }})\n", fields.join(", ")))
    }
}
/// Prefix acceptance mirrors the inspected validator. This does not certify rendering meaning.
pub fn validate_style(leaf: &str, style: &str) -> Result<(), String> {
    text(style)?;
    if style.is_empty() {
        return Ok(());
    }
    let workspace = leaf.starts_with("workspaces") || leaf.starts_with("specialWorkspace");
    let window = leaf.starts_with("window");
    let layer = leaf.starts_with("layers");
    let accepted = if window {
        style.starts_with("slide")
            || ["gnome", "gnomed"].contains(&style)
            || style.starts_with("popin")
    } else if workspace {
        ["slide", "slidevert", "fade"].contains(&style) || style.starts_with("slide")
    } else if leaf.ends_with("angle") {
        ["loop", "once"].contains(&style)
    } else if layer {
        style == "fade" || style.starts_with("slide") || style.starts_with("popin")
    } else {
        false
    };
    if !accepted {
        return Err("Style is not accepted for this animation node".into());
    }
    if (workspace && style.starts_with("slide")
        || ((window || layer) && style.starts_with("popin")))
        && style.contains('%')
    {
        let last = style
            .rfind(' ')
            .ok_or("Percentage style requires a separating space")?;
        let token = &style.as_bytes()[last + 1..];
        let token = &token[..token.len().saturating_sub(1)];
        let token = token.trim_ascii_start();
        let token = token.strip_prefix(b"+").unwrap_or(token);
        let end = token
            .iter()
            .enumerate()
            .find(|(i, c)| !(c.is_ascii_digit() || *i == 0 && **c == b'-'))
            .map_or(token.len(), |(i, _)| i);
        std::str::from_utf8(&token[..end])
            .map_err(|_| "Invalid style percentage")?
            .parse::<i32>()
            .map_err(|_| "Invalid style percentage")?;
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum Operation {
    Curve(Curve),
    Node(Node),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub id: String,
    pub operation: Operation,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub legacy: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub declarations: Vec<Declaration>,
}
impl Draft {
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) || self.declarations.len() > MAX_OPERATIONS
        {
            return Err(
                "Animation draft needs a known snapshot and at most 1024 declarations".into(),
            );
        }
        let mut ids = BTreeSet::new();
        let mut beziers: BTreeSet<String> = ["default".into(), "linear".into()].into();
        let mut springs: BTreeSet<String> = ["default".into()].into();
        let mut native_beziers: BTreeSet<String> = ["default".into(), "linear".into()].into();
        for declaration in &self.declarations {
            let id = &declaration.id;
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                || !ids.insert(id)
            {
                return Err("Invalid or duplicate animation declaration identity".into());
            }
            if declaration.legacy {
                if version != BASELINE {
                    return Err(
                        "Native animation declarations require the baseline snapshot".into(),
                    );
                }
                crate::legacy_animation::validate_operation(&declaration.operation)?;
                match &declaration.operation {
                    Operation::Curve(c) => {
                        native_beziers.insert(c.name.clone());
                    }
                    Operation::Node(n) if enabled(&n.enabled)? => {
                        if !n
                            .bezier
                            .as_ref()
                            .is_some_and(|name| native_beziers.contains(name))
                        {
                            return Err("Native Bézier must be defined earlier in the native declaration view".into());
                        }
                    }
                    _ => {}
                }
                continue;
            }
            match &declaration.operation {
                Operation::Curve(c) => {
                    c.validate()?;
                    if version == BASELINE
                        && matches!(
                            c.shape,
                            CurveKind::Spring {
                                dampening_alias: false,
                                ..
                            }
                        )
                    {
                        return Err(
                            "The baseline spring API requires the dampening spelling".into()
                        );
                    }
                    match c.shape {
                        CurveKind::Bezier { .. } => {
                            beziers.insert(c.name.clone());
                        }
                        CurveKind::Spring { .. } => {
                            springs.insert(c.name.clone());
                        }
                    }
                }
                Operation::Node(n) => {
                    n.validate(version)?;
                    if enabled(&n.enabled)? {
                        if let Some(name) = &n.bezier {
                            if !beziers.contains(name) {
                                return Err(format!(
                                    "Bézier {name} is not defined before this declaration"
                                ));
                            }
                        } else if let Some(name) = &n.spring {
                            if !springs.contains(name) {
                                return Err(format!(
                                    "Spring {name} is not defined before this declaration"
                                ));
                            }
                        }
                    }
                }
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES {
            return Err("Animation declarations exceed 1 MiB".into());
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut out = String::new();
        for declaration in &self.declarations {
            if declaration.legacy {
                return Err("Native animation declarations require explicit Lua conversion".into());
            }
            out.push_str(&match &declaration.operation {
                Operation::Curve(c) => c.export_lua()?,
                Operation::Node(n) => n.export_lua(version)?,
            });
        }
        Ok(out)
    }
    pub fn same_declarations(&self, other: &Self) -> bool {
        self.declarations.len() == other.declarations.len()
            && self
                .declarations
                .iter()
                .zip(&other.declarations)
                .all(|(a, b)| {
                    a.legacy == b.legacy
                        && if a.legacy {
                            match (
                                crate::legacy_animation::encode(a),
                                crate::legacy_animation::encode(b),
                            ) {
                                (Ok(a), Ok(b)) => a == b,
                                _ => a.operation == b.operation,
                            }
                        } else {
                            a.operation == b.operation
                        }
                })
    }
    pub fn resolve(&self, version: &str) -> Result<BTreeMap<String, Resolved>, String> {
        self.validate(version)?;
        if self.declarations.iter().any(|d| d.legacy) && self.declarations.iter().any(|d| !d.legacy)
        {
            return Err("Separate native and Lua drafts need separate inheritance review".into());
        }
        let nodes = catalogue(version);
        let mut own = BTreeMap::new();
        for n in &nodes {
            if let Some(value) = &n.initial {
                own.insert(n.name.clone(), (value.clone(), false));
            }
        }
        for d in &self.declarations {
            if let Operation::Node(n) = &d.operation {
                own.insert(n.leaf.clone(), (n.effective()?, true));
            }
        }
        let parents = nodes
            .iter()
            .map(|n| (n.name.as_str(), n.parent.as_deref()))
            .collect::<BTreeMap<_, _>>();
        let mut result = BTreeMap::new();
        for n in &nodes {
            let mut ancestor = n.name.as_str();
            let mut steps = 0;
            loop {
                if let Some((value, user)) = own.get(ancestor) {
                    result.insert(
                        n.name.clone(),
                        Resolved {
                            value: value.clone(),
                            origin: ancestor.into(),
                            user_override: *user,
                        },
                    );
                    break;
                }
                ancestor = parents
                    .get(ancestor)
                    .and_then(|p| *p)
                    .ok_or("Animation root has no initial configuration")?;
                steps += 1;
                if steps > nodes.len() {
                    return Err("Animation tree has a cycle".into());
                }
            }
        }
        Ok(result)
    }
    pub fn findings(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        let mut leaves = BTreeSet::new();
        let mut messages = vec![];
        for d in &self.declarations {
            match &d.operation {
                Operation::Curve(c) => {
                    let kind = matches!(c.shape, CurveKind::Spring { .. });
                    if !names.insert((d.legacy, kind, &c.name))
                        || c.name == "default"
                        || (!kind && c.name == "linear")
                    {
                        messages.push(format!("{}: named curve replaces an existing registry entry; referenced animations use its latest definition.",d.id));
                    }
                }
                Operation::Node(n) => {
                    if !leaves.insert((d.legacy, &n.leaf)) {
                        messages.push(format!(
                            "{}: later declaration replaces the whole node override.",
                            d.id
                        ));
                    }
                    if n.bezier
                        .as_ref()
                        .is_some_and(|s| s.starts_with("spring:") && s.len() > 7)
                    {
                        messages.push(format!("{}: this Bézier reference is interpreted as a spring at runtime because of its name prefix.",d.id));
                    }
                    if n.bezier.is_some() && n.spring.is_some() {
                        messages.push(format!(
                            "{}: Bézier takes precedence over the supplied spring reference.",
                            d.id
                        ));
                    }
                    if enabled(&n.enabled) == Ok(false) {
                        messages.push(format!("{}: disabled declaration uses speed 1/default curve/empty style; other retained fields are not consulted by the API.",d.id));
                    }
                }
            }
            if messages.len() >= 256 {
                messages.truncate(256);
                break;
            }
        }
        messages
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub value: Effective,
    pub origin: String,
    pub user_override: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue as scalars, lua, workspace::Workspace, Draft as ScalarDraft};
    fn parse(source: &str, version: &str) -> lua::LuaReport {
        lua::parse(source, version, &scalars()).unwrap()
    }
    #[test]
    fn static_models_match_independent_source_binding_tree_observations() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/animation-source-fixtures.json"))
                .unwrap();
        for version in [BASELINE, AUDITED_MAIN] {
            let label = if version == BASELINE {
                "baseline"
            } else {
                "current"
            };
            let cases = fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["version"] == label)
                .collect::<Vec<_>>();
            let defaults = cases.iter().find(|r| r["case"] == "defaults").unwrap()["tree"]
                .as_array()
                .unwrap();
            let source_nodes = defaults
                .iter()
                .map(|r| (r["name"].as_str().unwrap(), r))
                .collect::<BTreeMap<_, _>>();
            for node in catalogue(version) {
                assert_eq!(
                    serde_json::to_value(&node.parent).unwrap(),
                    source_nodes[node.name.as_str()]["parent"]
                );
            }
            for case in cases {
                let report = parse(case["source"].as_str().unwrap(), version);
                let accepted = case["accepted"].as_bool().unwrap();
                assert_eq!(
                    !report.opaque_effects, accepted,
                    "{label}: {} {:?}",
                    case["case"], report.diagnostics
                );
                if accepted {
                    let mut expected = source_nodes.clone();
                    for row in case["tree"].as_array().unwrap() {
                        expected.insert(row["name"].as_str().unwrap(), row);
                    }
                    let resolved = report.animations.resolve(version).unwrap();
                    for (name, actual) in resolved {
                        let row = expected[name.as_str()];
                        assert_eq!(
                            actual.origin,
                            row["origin"].as_str().unwrap(),
                            "{}:{name}",
                            case["case"]
                        );
                        assert_eq!(actual.value.enabled, row["enabled"].as_bool().unwrap());
                        assert_eq!(actual.value.speed, row["speed"].as_f64().unwrap());
                        assert_eq!(actual.value.curve, row["curve"].as_str().unwrap());
                        assert_eq!(actual.value.style, row["style"].as_str().unwrap());
                    }
                }
            }
        }
    }
    #[test]
    fn every_source_node_round_trips_and_whole_node_overrides_have_required_enabled() {
        for version in [BASELINE, AUDITED_MAIN] {
            let nodes = catalogue(version);
            assert_eq!(nodes.len(), 35);
            assert_eq!(nodes.iter().filter(|n| !n.internal).count(), 34);
            for info in nodes {
                let mut node = Node::new(&info.name);
                node.style = Some(String::new());
                let report = parse(&node.export_lua(version).unwrap(), version);
                assert!(
                    !report.opaque_effects,
                    "{} {:?}",
                    info.name, report.diagnostics
                );
                assert_eq!(
                    report.animations.declarations[0].operation,
                    Operation::Node(node)
                );
            }
            let r = parse(
                "hl.animation{leaf='windows',speed=8,bezier='default'}",
                version,
            );
            assert!(r.opaque_effects);
            assert!(r.animations.is_empty());
        }
    }
    #[test]
    fn named_curve_order_and_float32_spring_limits_are_enforced() {
        let source="hl.curve('ease',{type='bezier',points={{false,1},{0.25,1}}});hl.animation{leaf='windows',enabled=1,speed='5',bezier='ease'};hl.curve('bounce',{type='spring',stiffness=250,dampening='25',mass=1});hl.animation{leaf='windowsIn',enabled=true,speed=8,spring='bounce'}";
        let report = parse(source, BASELINE);
        assert!(!report.opaque_effects, "{:?}", report.diagnostics);
        assert_eq!(report.animations.declarations.len(), 4);
        let exported = report.animations.export_lua(BASELINE).unwrap();
        assert!(exported.contains("dampening"));
        assert!(parse(&exported, BASELINE)
            .animations
            .same_declarations(&report.animations));
        let r=parse("hl.animation{leaf='windows',enabled=true,speed=8,bezier='later'};hl.curve('later',{type='bezier',points={{0,0},{1,1}}})",BASELINE);
        assert!(r.opaque_effects);
        assert_eq!(r.animations.declarations.len(), 1);
        for amount in [0.5, 0.50000000001, 0., f64::INFINITY] {
            let curve = Curve {
                name: "spring".into(),
                shape: CurveKind::Spring {
                    stiffness: Value::Float(amount),
                    damping: Value::Float(25.),
                    mass: Value::Float(1.),
                    dampening_alias: true,
                },
            };
            assert!(curve.validate().is_err());
        }
        for coord in [-1., 2.] {
            let curve = Curve {
                name: "edge".into(),
                shape: CurveKind::Bezier {
                    points: [
                        [Value::Float(coord), Value::Float(coord)],
                        [Value::Float(0.), Value::Float(1.)],
                    ],
                },
            };
            assert!(curve.validate().is_ok());
        }
    }
    #[test]
    fn inheritance_uses_nearest_whole_node_override_and_removal_restores_defaults() {
        let mut draft = Draft::default();
        let defaults = draft.resolve(BASELINE).unwrap();
        assert_eq!(defaults["windowsIn"].origin, "global");
        assert!(!defaults["borderangle"].value.enabled);
        assert_eq!(defaults["__internal_fadeCTM"].value.curve, "linear");
        let mut parent = Node::new("windows");
        parent.speed = Some(Value::Float(3.));
        parent.style = Some("slide".into());
        draft.declarations.push(Declaration {
            legacy: false,
            id: "p".into(),
            operation: Operation::Node(parent),
        });
        let mut child = Node::new("windowsIn");
        child.enabled = Value::Bool(false);
        child.speed = Some(Value::Float(-2.));
        child.bezier = Some("unregistered-but-ignored".into());
        child.style = Some("ignored style".into());
        draft.declarations.push(Declaration {
            legacy: false,
            id: "c".into(),
            operation: Operation::Node(child),
        });
        let resolved = draft.resolve(BASELINE).unwrap();
        assert_eq!(resolved["windowsOut"].origin, "windows");
        assert_eq!(resolved["windowsOut"].value.speed, 3.);
        assert_eq!(resolved["windowsIn"].value.speed, 1.);
        assert_eq!(resolved["windowsIn"].value.curve, "default");
        assert_eq!(resolved["windowsIn"].value.style, "");
        assert!(!resolved["windowsIn"].value.enabled);
        draft.declarations.pop();
        assert_eq!(
            draft.resolve(BASELINE).unwrap()["windowsIn"].origin,
            "windows"
        );
        draft.declarations.clear();
        assert_eq!(draft.resolve(BASELINE).unwrap(), defaults);
    }
    #[test]
    fn timing_styles_and_registry_findings_do_not_claim_spring_duration_or_renderer_equivalence() {
        let node = Node::new("windows");
        assert_eq!(node.effective().unwrap().bezier_duration_ms(), Some(800.));
        let mut spring = node.clone();
        spring.bezier = None;
        spring.spring = Some("default".into());
        assert_eq!(spring.effective().unwrap().bezier_duration_ms(), None);
        for (leaf, style, accepted) in [
            ("windowsIn", "popin 75%", true),
            ("layersIn", "popin 50%", true),
            ("workspaces", "slidevert 120%", true),
            ("borderangle", "loop", true),
            ("fade", "slide", false),
            ("windows", "popin50%", false),
        ] {
            assert_eq!(validate_style(leaf, style).is_ok(), accepted);
        }
        let mut special = node;
        special.bezier = Some("spring:default".into());
        let draft = Draft {
            declarations: vec![Declaration {
                legacy: false,
                id: "prefix".into(),
                operation: Operation::Node(special),
            }],
        };
        assert!(draft.findings()[0].contains("interpreted as a spring"));
    }
    #[test]
    fn private_animation_storage_pins_the_source_tree_and_rejects_forged_shape() {
        let report = parse("hl.animation{leaf='windows',enabled=false}", BASELINE);
        let workspace =
            Workspace::new(ScalarDraft::new(BASELINE)).with_animations(report.animations.clone());
        let data = workspace.to_json(&scalars()).unwrap();
        assert_eq!(
            Workspace::from_json(std::str::from_utf8(&data).unwrap(), &scalars())
                .unwrap()
                .animations,
            report.animations
        );
        let mut bad = workspace;
        bad.animation_schema_sha256 = Some("0".repeat(64));
        assert!(bad.to_json(&scalars()).is_err());
        let mut node = Node::new("windows");
        node.speed = Some(Value::Float(1e-60));
        assert!(node.validate(BASELINE).is_err());
        node.speed = Some(Value::Float(101.));
        assert!(node.validate(BASELINE).is_err());
        assert!(parse("hl.animation{leaf='windows',enabled=true,speed=untrusted(),bezier='default'};hl.config{general={border_size=8}}",BASELINE).draft.values.is_empty());
        assert!(parse("hl.curve('x',{type='bezier',points={{0,0},{1,1}},extra=untrusted()});hl.config{general={border_size=8}}",BASELINE).draft.values.is_empty());
    }
    #[test]
    fn large_programs_do_not_require_repeated_projection_validation() {
        let source = (0..1024)
            .map(|_| "hl.animation{leaf='windows',enabled=false}\n")
            .collect::<String>();
        let report = parse(&source, BASELINE);
        assert!(!report.opaque_effects);
        assert_eq!(report.animations.declarations.len(), 1024);
        assert!(
            !report.animations.resolve(BASELINE).unwrap()["windows"]
                .value
                .enabled
        );
    }
}
