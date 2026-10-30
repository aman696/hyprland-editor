//! Captured legacy bytes and restricted projections, with no restored path access.
use hyprland_editor_core::{
    import, legacy_monitor, legacy_session, session, Draft, Setting, BASELINE,
};
#[derive(Debug)]
pub(super) struct Document {
    pub content: String,
    pub report: import::ImportReport,
    pub session: legacy_session::Report,
    pub monitors: legacy_monitor::Report,
}
impl Document {
    pub fn new(content: String, catalogue: &[Setting]) -> Result<Self, String> {
        #[cfg(not(test))]
        let parsed = match super::worker::run(super::worker::Request::ParseLegacy {
            source: content.clone(),
        })? {
            super::worker::Response::LegacyParsed(report) => *report,
            _ => return Err("Unexpected legacy parser response".into()),
        };
        #[cfg(test)]
        let parsed =
            hyprland_editor_core::legacy::Document::new(content.clone(), catalogue)?.into_report();
        if parsed.monitors.schema_sha256 != legacy_monitor::schema_hash()
            || parsed.monitors.source_sha256 != parsed.source_sha256
            || parsed.monitors.sites.len() != parsed.monitors.rules.len()
            || parsed
                .monitors
                .sites
                .iter()
                .chain(&parsed.session.sites)
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
        hyprland_editor_core::monitor::validate(&parsed.monitors.rules, BASELINE)?;
        let mut report = parsed.scalar;
        let session_lines: std::collections::BTreeSet<_> =
            parsed.session.sites.iter().map(|site| site.line).collect();
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
        })
    }
    pub fn edited_with_session(
        &self,
        draft: &Draft,
        session: &session::Draft,
        monitors: &[hyprland_editor_core::monitor::Rule],
    ) -> Result<String, String> {
        match super::worker::run(super::worker::Request::CopyLegacy {
            source: self.content.clone(),
            draft: draft.clone(),
            session: session.clone(),
            monitors: monitors.to_vec(),
        })? {
            super::worker::Response::Copy(source) => Ok(source),
            _ => Err("Unexpected legacy copy response".into()),
        }
    }
}
