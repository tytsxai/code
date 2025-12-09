use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use code_core::CodexConversation;
use code_core::protocol::Op;
use code_core::protocol::ReviewDecision;
use mcp_types::ElicitRequest;
use mcp_types::ElicitRequestParamsRequestedSchema;
use mcp_types::JSONRPCErrorError;
use mcp_types::ModelContextProtocolRequest;
use mcp_types::RequestId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tokio::time::timeout;
use tracing::{error, warn};

use crate::code_tool_runner::INVALID_PARAMS_ERROR_CODE;
use crate::outgoing_message::OutgoingMessageSender;

const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Conforms to [`mcp_types::ElicitRequestParams`] so that it can be used as the
/// `params` field of an [`ElicitRequest`].
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecApprovalElicitRequestParams {
    // These fields are required so that `params`
    // conforms to ElicitRequestParams.
    pub message: String,

    #[serde(rename = "requestedSchema")]
    pub requested_schema: ElicitRequestParamsRequestedSchema,

    // These are additional fields the client can use to
    // correlate the request with the codex tool call.
    pub code_elicitation: String,
    pub code_mcp_tool_call_id: String,
    pub code_event_id: String,
    pub code_call_id: String,
    pub code_command: Vec<String>,
    pub code_cwd: PathBuf,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_exec_approval_request(
    command: Vec<String>,
    cwd: PathBuf,
    outgoing: Arc<OutgoingMessageSender>,
    codex: Arc<CodexConversation>,
    request_id: RequestId,
    tool_call_id: String,
    event_id: String,
    call_id: String,
) {
    let escaped_command =
        shlex::try_join(command.iter().map(String::as_str)).unwrap_or_else(|_| command.join(" "));
    let message = format!(
        "Allow Codex to run `{escaped_command}` in `{cwd}`?",
        cwd = cwd.to_string_lossy()
    );

    let params = ExecApprovalElicitRequestParams {
        message,
        requested_schema: ElicitRequestParamsRequestedSchema {
            r#type: "object".to_string(),
            properties: json!({}),
            required: None,
        },
        code_elicitation: "exec-approval".to_string(),
        code_mcp_tool_call_id: tool_call_id.clone(),
        code_event_id: event_id.clone(),
        code_call_id: call_id.clone(),
        code_command: command,
        code_cwd: cwd,
    };
    let params_json = match serde_json::to_value(&params) {
        Ok(value) => value,
        Err(err) => {
            let message = format!("Failed to serialize ExecApprovalElicitRequestParams: {err}");
            error!("{message}");

            outgoing
                .send_error(
                    request_id.clone(),
                    JSONRPCErrorError {
                        code: INVALID_PARAMS_ERROR_CODE,
                        message,
                        data: None,
                    },
                )
                .await;

            return;
        }
    };

    let on_response = outgoing
        .send_request(ElicitRequest::METHOD, Some(params_json))
        .await;

    // Listen for the response on a separate task so we don't block the main agent loop.
    {
        let codex = codex.clone();
        // Correlate by call_id for core pending approvals
        let approval_id = call_id.clone();
        tokio::spawn(async move {
            on_exec_approval_response(approval_id, on_response, codex).await;
        });
    }
}

async fn on_exec_approval_response(
    approval_id: String,
    receiver: tokio::sync::oneshot::Receiver<mcp_types::Result>,
    codex: Arc<CodexConversation>,
) {
    let value = match timeout(APPROVAL_TIMEOUT, receiver).await {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => {
            error!("exec approval request failed: {err:?}");
            if let Err(submit_err) = codex
                .submit(Op::ExecApproval {
                    id: approval_id.clone(),
                    decision: ReviewDecision::Denied,
                })
                .await
            {
                error!("failed to submit denied ExecApproval after request failure: {submit_err}");
            }
            return;
        }
        Err(_) => {
            warn!(
                "exec approval request timed out after {:?} (call_id={})",
                APPROVAL_TIMEOUT, approval_id
            );
            if let Err(err) = codex
                .submit(Op::ExecApproval {
                    id: approval_id,
                    decision: ReviewDecision::Denied,
                })
                .await
            {
                error!("failed to submit denied ExecApproval after timeout: {err}");
            }
            return;
        }
    };

    let decision = code_protocol::protocol::ReviewDecision::from_value(&value).unwrap_or_else(|| {
        error!("failed to deserialize exec approval response (value={value:?}); denying");
        code_protocol::protocol::ReviewDecision::Denied
    });
    let decision: ReviewDecision = decision.into();

    if let Err(err) = codex
        .submit(Op::ExecApproval {
            id: approval_id,
            decision,
        })
        .await
    {
        error!("failed to submit ExecApproval: {err}");
    }
}
