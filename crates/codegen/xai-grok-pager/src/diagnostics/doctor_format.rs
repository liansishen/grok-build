//! In-TUI `/doctor` report formatting.

use xai_grok_i18n::{t, t_fmt};

use super::{
    DataControlFact, DiagnosticReport, FindingDisposition, NewlineFact, RuntimeFact, VoiceFacts,
};
use crate::clipboard::{ClipboardDelivery, NativeClipboardPreflight};
use crate::host::{DisplayServer, HostOs};

pub fn format_doctor(report: &DiagnosticReport) -> String {
    let facts = &report.facts;
    let mut out = String::new();
    out.push_str(&format!("{}\n", t("doctor_fmt.section_environment")));
    out.push_str(&format!("  {}     {}\n", t("doctor_fmt.label_terminal"), facts.terminal));
    if let RuntimeFact::Available(xtversion) = &facts.xtversion {
        out.push_str(&format!("  {}    {xtversion}\n", t("doctor_fmt.label_xtversion")));
    }
    out.push_str(&format!("  {}  {}\n", t("doctor_fmt.label_multiplexer"), facts.multiplexer));
    if let Some(byobu) = facts.byobu {
        out.push_str(&format!("  {}        {byobu}\n", t("doctor_fmt.label_byobu")));
    }
    out.push_str(&format!(
        "  {}          {}\n",
        t("doctor_fmt.label_ssh"),
        if facts.ssh { t("doctor_fmt.value_yes") } else { t("doctor_fmt.value_no") }
    ));
    let color_level = match &facts.color.level {
        RuntimeFact::Available(level) => Some(*level),
        RuntimeFact::NoReply | RuntimeFact::Unavailable => None,
    };
    if let Some(color_level) = color_level {
        out.push_str(&format!("  {}        {}\n", t("doctor_fmt.label_color"), color_level.as_str()));
    }
    if color_level.is_some() && facts.color.available_themes.len() == facts.color.total_themes {
        out.push_str(&format!("  {}       {}\n", t("doctor_fmt.label_themes"), t("doctor_fmt.value_all")));
    } else if color_level.is_some() {
        let themes = facts
            .color
            .available_themes
            .iter()
            .map(|theme| theme.display_name())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "  {}       {}/{}: {themes}\n",
            t("doctor_fmt.label_themes"),
            facts.color.available_themes.len(),
            facts.color.total_themes
        ));
    }
    if let Some(keyboard) = &facts.keyboard {
        let rescue = if keyboard.os == HostOs::Macos {
            t("doctor_fmt.os_rescue_active")
        } else {
            t("doctor_fmt.os_rescue_unavailable")
        };
        out.push_str(&format!(
            "  {}     {} ({rescue})\n",
            t("doctor_fmt.label_keyboard"),
            keyboard.modifier_delivery.label()
        ));
    }
    if let Some(newline) = &facts.newline {
        let detail = match newline {
            NewlineFact::Vte {
                version: Some(version),
            } => t_fmt("doctor_fmt.newline_vte_version", &[("version", version)]),
            NewlineFact::Vte { version: None } => {
                t("doctor_fmt.newline_vte_legacy").to_owned()
            }
            NewlineFact::XtermJs { terminal } => {
                t_fmt("doctor_fmt.newline_xtermjs", &[("terminal", &terminal.to_string())])
            }
            NewlineFact::NoKittyKeyboardProtocol => {
                t("doctor_fmt.newline_no_kitty").to_owned()
            }
        };
        out.push_str(&format!("  {}      Alt+Enter ({detail})\n", t("doctor_fmt.label_newline")));
    }

    let clipboard = &facts.clipboard;
    let native = match clipboard.native_preflight {
        NativeClipboardPreflight::LocalAvailable => {
            t_fmt("doctor_fmt.native_local", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::RemoteOnly if clipboard.container_no_display => {
            t_fmt("doctor_fmt.native_container", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::RemoteOnly => {
            t_fmt("doctor_fmt.native_remote", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::Unavailable => t("doctor_fmt.value_unavailable").to_owned(),
        NativeClipboardPreflight::Disabled => t("doctor_fmt.value_off").to_owned(),
    };
    out.push_str(&format!("\n{}\n", t("doctor_fmt.section_clipboard")));
    out.push_str(&format!("  {}       {native}\n", t("doctor_fmt.label_native")));
    out.push_str(&format!(
        "  {}         {}\n",
        t("doctor_fmt.label_tmux"),
        if clipboard.tmux_route { t("doctor_fmt.value_on") } else { t("doctor_fmt.value_off") }
    ));
    out.push_str(&format!(
        "  {}       {}\n",
        t("doctor_fmt.label_osc52"),
        if clipboard.osc52_route {
            clipboard.osc52_capability.label()
        } else {
            t("doctor_fmt.value_off")
        }
    ));
    out.push_str(&format!(
        "  {}         {}\n",
        t("doctor_fmt.label_wrap"),
        if clipboard.wrap_sink { t("doctor_fmt.value_on") } else { t("doctor_fmt.value_off") }
    ));
    if clipboard.display_server == DisplayServer::Wayland {
        out.push_str(&format!(
            "  {} {}\n",
            t("doctor_fmt.label_data_control"),
            if clipboard.data_control == DataControlFact::Available {
                t("doctor_fmt.value_on")
            } else {
                t("doctor_fmt.value_off")
            }
        ));
    }
    let status = match clipboard.delivery {
        ClipboardDelivery::Confirmed => t("doctor_fmt.value_confirmed"),
        ClipboardDelivery::Unverified => t("doctor_fmt.value_unverified"),
        ClipboardDelivery::Failed => t("doctor_fmt.value_unavailable"),
    };
    out.push_str(&format!("  {}       {status}\n", t("doctor_fmt.label_status")));

    if let Some(voice) = &facts.voice {
        out.push_str(&format!("\n{}\n", t("doctor_fmt.section_voice")));
        match voice {
            VoiceFacts::Device { name, detail } => {
                out.push_str(&format!("  {}   {name} ({detail})\n", t("doctor_fmt.label_microphone")));
            }
            VoiceFacts::Missing { .. } => {
                out.push_str(&format!("  {}   {}\n", t("doctor_fmt.label_microphone"), t("doctor_fmt.microphone_none")));
            }
        }
    }

    format_findings(report, &mut out);
    out
}

fn format_findings(report: &DiagnosticReport, out: &mut String) {
    let issues = report
        .findings
        .iter()
        .filter(|finding| finding.disposition == FindingDisposition::Issue)
        .collect::<Vec<_>>();
    if issues.is_empty() {
        if report.issue_count() == 0 {
            out.push_str(&format!("\n{}\n", t("doctor_fmt.no_issues")));
        } else {
            out.push_str(&format!("\n{}\n", t("doctor_fmt.issue_in_clipboard_status")));
        }
    } else {
        out.push_str(&format!("\n{}\n", t_fmt("doctor_fmt.issues_count", &[("count", &issues.len().to_string())])));
        for finding in issues {
            format_finding(out, finding);
        }
    }

    let recommendations = report
        .findings
        .iter()
        .filter(|finding| finding.disposition == FindingDisposition::Recommendation)
        .collect::<Vec<_>>();
    if !recommendations.is_empty() {
        out.push_str(&format!("\n{}\n", t("doctor_fmt.section_recommendations")));
        for finding in recommendations {
            format_finding(out, finding);
        }
    }
}

fn format_finding(out: &mut String, finding: &super::DiagnosticFinding) {
    let marker = match finding.disposition {
        FindingDisposition::Issue => "!",
        FindingDisposition::Recommendation => "i",
    };
    out.push_str(&format!(
        "\n  {marker} {}  {}\n",
        finding.id, finding.message
    ));
    if let Some(automatic) = finding.automatic_remediation {
        let command = super::human_fix_command(automatic.fix_id)
            .unwrap_or_else(|| automatic.command.to_owned());
        out.push_str(&format!("      {}\n", t_fmt("doctor_fmt.automatic_setup", &[("command", &command)])));
    }
    if let Some(remediation) = &finding.remediation {
        match (&remediation.config_path, &finding.automatic_remediation) {
            (Some(path), _) => {
                out.push_str(&format!("      {}\n", t_fmt("doctor_fmt.add_to_path", &[("fix", &remediation.fix), ("path", path)])));
            }
            (None, Some(_)) => {
                out.push_str(&format!("      {}\n", t_fmt("doctor_fmt.one_off", &[("fix", &remediation.fix)])));
            }
            (None, None) => {
                out.push_str(&format!("      {}\n", t_fmt("doctor_fmt.run_fix", &[("fix", &remediation.fix)])));
            }
        }
    }
    if let Some(note) = &finding.note {
        out.push_str(&format!("      {}\n", t_fmt("doctor_fmt.note", &[("note", note)])));
    }
}

#[cfg(test)]
#[path = "doctor_format_tests.rs"]
mod tests;
