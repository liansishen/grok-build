//! Pure dispatch preparation for minimal-mode external prompt editing.

use crate::app::actions::Effect;
use crate::app::agent_view::ExternalPromptEditorAccess;
use crate::app::app_view::{ActiveView, AppView, VoiceTarget};
use crate::app::external_editor::{
    PendingEditorRequest, attachment_message, paste_message, report_prompt_failure, voice_message,
};

pub(super) fn dispatch_edit_prompt_external(app: &mut AppView) -> Vec<Effect> {
    if !app.screen_mode.is_minimal() || app.pending_editor.is_some() {
        return vec![];
    }
    let ActiveView::Agent(agent_id) = app.active_view else {
        return vec![];
    };
    let Some(agent) = app.agents.get(&agent_id) else {
        return vec![];
    };
    let access = agent.external_prompt_editor_access(true);
    if app.voice_recording_target() == Some(VoiceTarget::Agent(agent_id)) {
        report_prompt_failure(app, agent_id, voice_message());
        return vec![];
    }
    match access {
        ExternalPromptEditorAccess::OwnedElsewhere => return vec![],
        ExternalPromptEditorAccess::PastePending => {
            report_prompt_failure(app, agent_id, paste_message());
            return vec![];
        }
        ExternalPromptEditorAccess::Attachments => {
            report_prompt_failure(app, agent_id, attachment_message());
            return vec![];
        }
        ExternalPromptEditorAccess::Ready => {}
    }

    app.pending_editor = Some(PendingEditorRequest::PromptDraft {
        agent_id,
        original_text: app.agents[&agent_id].prompt.text().to_owned(),
    });
    vec![]
}
