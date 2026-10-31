//! Captured legacy bytes and restricted projections, with no restored path access.
use hyprland_editor_core::{import, legacy_monitor, legacy_session, Setting, BASELINE};
#[derive(Debug)]
pub(super) struct Document {
    pub content: String,
    pub report: import::ImportReport,
    pub session: legacy_session::Report,
    pub monitors: legacy_monitor::Report,
    pub bindings: hyprland_editor_core::legacy_binding::Report,
    pub workspaces: hyprland_editor_core::legacy_workspace::Report,
    pub animations: hyprland_editor_core::legacy_animation::Report,
    pub gestures: hyprland_editor_core::legacy_gesture::Report,
}
impl Document {
    pub fn new(content: String, catalogue: &[Setting]) -> Result<Self, String> {
        Self::new_with_binding_mode(content, catalogue, true)
    }
    pub fn new_with_binding_mode(
        content: String,
        catalogue: &[Setting],
        local_variables: bool,
    ) -> Result<Self, String> {
        #[cfg(not(test))]
        let parsed = match super::worker::run(super::worker::Request::ParseLegacy {
            source: content.clone(),
            binding_variables: local_variables,
        })? {
            super::worker::Response::LegacyParsed(report) => *report,
            _ => return Err("Unexpected legacy parser response".into()),
        };
        #[cfg(test)]
        let parsed = hyprland_editor_core::legacy::Document::new_with_binding_mode(
            content.clone(),
            catalogue,
            local_variables,
        )?
        .into_report();
        if parsed.bindings.schema_sha256
            != (if local_variables {
                hyprland_editor_core::legacy_binding::schema_hash()
            } else {
                hyprland_editor_core::legacy_binding::literal_schema_hash()
            })
            || parsed.bindings.draft.local_variables != local_variables
            || parsed.bindings.source_sha256 != parsed.source_sha256
            || parsed.bindings.sites.len() != parsed.bindings.draft.entries.len()
            || parsed.gestures.schema_sha256 != hyprland_editor_core::legacy_gesture::schema_hash()
            || parsed.gestures.source_sha256 != parsed.source_sha256
            || parsed.gestures.sites.len() != parsed.gestures.draft.declarations.len()
            || parsed.animations.schema_sha256
                != hyprland_editor_core::legacy_animation::schema_hash()
            || parsed.animations.source_sha256 != parsed.source_sha256
            || parsed.animations.sites.len() != parsed.animations.draft.declarations.len()
            || parsed.workspaces.schema_sha256
                != hyprland_editor_core::legacy_workspace::schema_hash()
            || parsed.workspaces.source_sha256 != parsed.source_sha256
            || parsed.workspaces.sites.len() != parsed.workspaces.rules.len()
            || parsed.monitors.schema_sha256 != legacy_monitor::schema_hash()
            || parsed.monitors.source_sha256 != parsed.source_sha256
            || parsed.monitors.sites.len() != parsed.monitors.rules.len()
            || parsed
                .monitors
                .sites
                .iter()
                .chain(&parsed.gestures.sites)
                .chain(&parsed.animations.sites)
                .chain(&parsed.workspaces.sites)
                .chain(&parsed.session.sites)
                .chain(&parsed.bindings.sites)
                .any(|site| content.get(site.statement.clone()).is_none())
            || parsed.session.schema_sha256 != legacy_session::schema_hash()
            || parsed.source_sha256 != hyprland_editor_core::lua::source_hash(&content)
            || parsed.session.source_sha256 != parsed.source_sha256
            || parsed.scalar.draft.version != BASELINE
            || parsed.session.sites.len() != parsed.session.draft.entries.len()
        {
            return Err("Legacy projection does not match the captured source".into());
        }
        parsed.scalar.draft.validate(catalogue)?;
        parsed.session.draft.validate(BASELINE)?;
        parsed.bindings.draft.validate(BASELINE)?;
        if (local_variables
            && parsed.bindings.draft.source_links.len() != parsed.bindings.sites.len())
            || (!local_variables && !parsed.bindings.draft.source_links.is_empty())
            || parsed
                .bindings
                .draft
                .source_links
                .iter()
                .zip(&parsed.bindings.sites)
                .any(|(link, site)| *link != Some(site.line))
        {
            return Err("Binding source references do not match captured declarations".into());
        }
        let binding_lines: std::collections::BTreeSet<_> =
            parsed.bindings.sites.iter().map(|site| site.line).collect();
        if parsed
            .bindings
            .derived_sites
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
            || parsed
                .bindings
                .derived_sites
                .iter()
                .any(|line| !binding_lines.contains(line))
        {
            return Err("Variable-derived binding sites do not match captured declarations".into());
        }
        hyprland_editor_core::monitor::validate(&parsed.monitors.rules, BASELINE)?;
        hyprland_editor_core::workspace_rule::validate(&parsed.workspaces.rules, BASELINE)?;
        parsed.animations.draft.validate(BASELINE)?;
        parsed.gestures.draft.validate(BASELINE)?;
        let mut report = parsed.scalar;
        let session_lines: std::collections::BTreeSet<_> = parsed
            .session
            .sites
            .iter()
            .chain(&parsed.bindings.sites)
            .chain(&parsed.workspaces.sites)
            .chain(&parsed.animations.sites)
            .chain(&parsed.gestures.sites)
            .map(|site| site.line)
            .collect();
        let monitor_lines: Vec<_> = parsed
            .monitors
            .sites
            .iter()
            .map(|site| {
                (
                    site.line,
                    site.line
                        + content[site.statement.clone()]
                            .bytes()
                            .filter(|b| *b == b'\n')
                            .count(),
                )
            })
            .collect();
        report.skipped.retain(|line| {
            let index = monitor_lines.partition_point(|(start, _)| start <= line);
            !session_lines.contains(line) && (index == 0 || *line > monitor_lines[index - 1].1)
        });
        Ok(Self {
            content,
            report,
            session: parsed.session,
            monitors: parsed.monitors,
            bindings: parsed.bindings,
            workspaces: parsed.workspaces,
            animations: parsed.animations,
            gestures: parsed.gestures,
        })
    }
    pub fn edited_copy(
        &self,
        workspace: &hyprland_editor_core::workspace::Workspace,
    ) -> Result<String, String> {
        match super::worker::run(super::worker::Request::CopyLegacy {
            source: self.content.clone(),
            draft: workspace.draft.clone(),
            session: workspace.session.clone(),
            monitors: workspace.monitors.clone(),
            bindings: workspace.bindings.legacy.clone().unwrap_or_default(),
            workspaces: workspace.workspace_rules.clone(),
            animations: workspace.animations.clone(),
            gestures: workspace.gestures.clone(),
        })? {
            super::worker::Response::Copy(source) => Ok(source),
            _ => Err("Unexpected legacy copy response".into()),
        }
    }
}
