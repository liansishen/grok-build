use xai_grok_i18n::{t, t_fmt};

use crate::clipboard::{ClipboardDelivery, NativeClipboardPreflight};
use crate::diagnostics::{
    DataControlFact, DiagnosticFinding, DiagnosticReport, FindingDisposition, NewlineFact,
    ProbeStatus, RuntimeFact, VoiceFacts,
};
use crate::host::{DisplayServer, HostOs};

pub(super) fn format(report: &DiagnosticReport) -> String {
    let facts = &report.facts;
    let mut out = String::from(t("doctor_cli.title"));
    out.push_str(t("doctor_cli.section_environment"));

    fact(&mut out, t("doctor_cli.label_terminal"), &facts.terminal.to_string());
    match &facts.xtversion {
        RuntimeFact::Available(value) => fact(&mut out, t("doctor_cli.label_terminal_version"), value),
        RuntimeFact::NoReply => unavailable(&mut out, t("doctor_cli.label_terminal_version"), t("doctor_cli.value_no_reply")),
        RuntimeFact::Unavailable => unavailable(&mut out, t("doctor_cli.label_terminal_version"), t("doctor_cli.value_unavailable")),
    }
    fact(&mut out, t("doctor_cli.label_multiplexer"), &facts.multiplexer.to_string());
    if let Some(byobu) = facts.byobu {
        fact(&mut out, t("doctor_cli.label_byobu"), &byobu.to_string());
    }
    fact(&mut out, t("doctor_cli.label_ssh"), if facts.ssh { t("doctor_cli.value_yes") } else { t("doctor_cli.value_no") });
    match &facts.color.level {
        RuntimeFact::Available(level) => {
            fact(&mut out, t("doctor_cli.label_color"), level.as_str());
            let themes = if facts.color.available_themes.len() == facts.color.total_themes {
                t("doctor_cli.value_all").to_owned()
            } else {
                format!(
                    "{}/{}: {}",
                    facts.color.available_themes.len(),
                    facts.color.total_themes,
                    facts
                        .color
                        .available_themes
                        .iter()
                        .map(|theme| theme.display_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            fact(&mut out, t("doctor_cli.label_themes"), &themes);
        }
        RuntimeFact::NoReply | RuntimeFact::Unavailable => {
            unavailable(&mut out, t("doctor_cli.label_color"), t("doctor_cli.value_unavailable"));
            unavailable(&mut out, t("doctor_cli.label_themes"), t("doctor_cli.value_unavailable"));
        }
    }

    if let Some(keyboard) = &facts.keyboard {
        let rescue = if keyboard.os == HostOs::Macos {
            t("doctor_cli.os_rescue_active")
        } else {
            t("doctor_cli.os_rescue_unavailable")
        };
        fact(
            &mut out,
            t("doctor_cli.label_keyboard"),
            &format!("{} ({rescue})", keyboard.modifier_delivery.label()),
        );
    }
    if let Some(newline) = &facts.newline {
        fact(&mut out, t("doctor_cli.label_newline"), &format_newline(newline));
    }

    let clipboard = &facts.clipboard;
    let native = match clipboard.native_preflight {
        NativeClipboardPreflight::LocalAvailable => {
            t_fmt("doctor_cli.native_local", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::RemoteOnly if clipboard.container_no_display => {
            t_fmt("doctor_cli.native_container", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::RemoteOnly => {
            t_fmt("doctor_cli.native_remote", &[("tool", &clipboard.native_tool)])
        }
        NativeClipboardPreflight::Unavailable => t("doctor_cli.value_unavailable").to_owned(),
        NativeClipboardPreflight::Disabled => t("doctor_cli.value_off").to_owned(),
    };
    out.push_str(t("doctor_cli.section_clipboard"));
    fact(&mut out, t("doctor_cli.label_native"), &native);
    fact(
        &mut out,
        t("doctor_cli.label_tmux"),
        if clipboard.tmux_route { t("doctor_cli.value_on") } else { t("doctor_cli.value_off") },
    );
    fact(
        &mut out,
        t("doctor_cli.label_osc52"),
        if clipboard.osc52_route {
            clipboard.osc52_capability.label()
        } else {
            t("doctor_cli.value_off")
        },
    );
    fact(
        &mut out,
        t("doctor_cli.label_ssh_wrap"),
        if clipboard.wrap_sink { t("doctor_cli.value_on") } else { t("doctor_cli.value_off") },
    );
    if clipboard.display_server == DisplayServer::Wayland {
        match clipboard.data_control {
            DataControlFact::Available => fact(&mut out, t("doctor_cli.label_data_control"), t("doctor_cli.value_on")),
            DataControlFact::Missing => fact(&mut out, t("doctor_cli.label_data_control"), t("doctor_cli.value_off")),
            DataControlFact::Unavailable => unavailable(&mut out, t("doctor_cli.label_data_control"), t("doctor_cli.value_unavailable")),
            DataControlFact::Error => {
                let detail = report
                    .probe_notes
                    .iter()
                    .find(|note| note.probe == "wayland.data-control")
                    .and_then(|note| note.message.as_deref());
                match detail {
                    Some(message) => {
                        unavailable(&mut out, t("doctor_cli.label_data_control"), &t_fmt("doctor_cli.value_error_detail", &[("message", message)]))
                    }
                    None => unavailable(&mut out, t("doctor_cli.label_data_control"), t("doctor_cli.value_error")),
                }
            }
            DataControlFact::NotApplicable => {}
        }
    }
    let status = match clipboard.delivery {
        ClipboardDelivery::Confirmed => t("doctor_cli.value_confirmed"),
        ClipboardDelivery::Unverified => t("doctor_cli.value_unverified"),
        ClipboardDelivery::Failed => t("doctor_cli.value_unavailable"),
    };
    fact(&mut out, t("doctor_cli.label_status"), status);

    if let Some(voice) = &facts.voice {
        out.push_str(t("doctor_cli.section_voice"));
        match voice {
            VoiceFacts::Device { name, detail } => {
                fact(&mut out, t("doctor_cli.label_microphone"), &format!("{name} ({detail})"));
            }
            VoiceFacts::Missing { error } => {
                fact(&mut out, t("doctor_cli.label_microphone"), &t_fmt("doctor_cli.microphone_none_detected", &[("error", error)]));
            }
        }
    }

    if !report.findings.is_empty() {
        out.push_str(t("doctor_cli.section_findings"));
        for finding in &report.findings {
            format_finding(&mut out, finding);
        }
    }

    let visible_notes = report
        .probe_notes
        .iter()
        .filter(|note| !fact_already_shows_probe(note.probe));
    let mut notes = visible_notes.peekable();
    if notes.peek().is_some() {
        out.push_str(t("doctor_cli.section_checks_not_completed"));
        for note in notes {
            let message = match &note.message {
                Some(message) => format!("{}: {message}", probe_status(note.status)),
                None => probe_status(note.status).to_owned(),
            };
            row(&mut out, "?", note.probe, &message);
        }
    }

    if report
        .probe_notes
        .iter()
        .any(crate::diagnostics::probe_requires_live_tui)
    {
        out.push_str(t("doctor_cli.section_needs_session"));
        out.push_str(&format!("  {}\n", t("doctor_cli.live_tui_probe_cta")));
    }

    let issues = report.issue_count();
    let recommendations = report.recommendation_count();
    out.push('\n');
    out.push_str(&format!(
        "{} {}, {} {}\n",
        issues,
        plural(issues, t("doctor_cli.issue_singular"), t("doctor_cli.issue_plural")),
        recommendations,
        plural(recommendations, t("doctor_cli.recommendation_singular"), t("doctor_cli.recommendation_plural"))
    ));
    out
}

fn fact_already_shows_probe(probe: &str) -> bool {
    matches!(
        probe,
        "runtime.xtversion" | "terminal.color" | "wayland.data-control"
    )
}

fn fact(out: &mut String, label: &str, value: &str) {
    row(out, "·", label, value);
}

fn unavailable(out: &mut String, label: &str, value: &str) {
    row(out, "?", label, value);
}

fn row(out: &mut String, marker: &str, label: &str, value: &str) {
    out.push_str(&format!("  {marker} {label:<28} {value}\n"));
}

fn format_finding(out: &mut String, finding: &DiagnosticFinding) {
    let marker = match finding.disposition {
        FindingDisposition::Issue => "!",
        FindingDisposition::Recommendation => "i",
    };
    row(out, marker, &finding.id.to_string(), &finding.message);
    if let Some(automatic) = finding.automatic_remediation {
        let command = crate::diagnostics::human_fix_command(automatic.fix_id)
            .unwrap_or_else(|| automatic.command.to_owned());
        out.push_str(&format!("    → {}\n", t_fmt("doctor_cli.automatic_setup", &[("command", &command)])));
    }
    if let Some(remediation) = &finding.remediation {
        let instruction = match (&remediation.config_path, &finding.automatic_remediation) {
            (Some(path), _) => t_fmt("doctor_cli.add_to_path", &[("fix", &remediation.fix), ("path", path)]),
            (None, Some(_)) => t_fmt("doctor_cli.one_off", &[("fix", &remediation.fix)]),
            (None, None) => t_fmt("doctor_cli.run_fix", &[("fix", &remediation.fix)]),
        };
        out.push_str(&format!("    → {instruction}\n"));
    }
    if let Some(note) = &finding.note {
        out.push_str(&format!("      {note}\n"));
    }
}

fn format_newline(newline: &NewlineFact) -> String {
    let detail = match newline {
        NewlineFact::Vte {
            version: Some(version),
        } => t_fmt("doctor_cli.newline_vte_version", &[("version", version)]),
        NewlineFact::Vte { version: None } => {
            t("doctor_cli.newline_vte_legacy").to_owned()
        }
        NewlineFact::XtermJs { terminal } => {
            t_fmt("doctor_cli.newline_xtermjs", &[("terminal", &terminal.to_string())])
        }
        NewlineFact::NoKittyKeyboardProtocol => {
            t("doctor_cli.newline_no_kitty").to_owned()
        }
    };
    t_fmt("doctor_cli.newline_alt_enter", &[("detail", &detail)])
}

fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

fn probe_status(status: ProbeStatus) -> &'static str {
    match status {
        ProbeStatus::Unsupported => t("doctor_cli.probe_unsupported"),
        ProbeStatus::Unavailable => t("doctor_cli.probe_unavailable"),
        ProbeStatus::Error => t("doctor_cli.probe_error"),
    }
}
