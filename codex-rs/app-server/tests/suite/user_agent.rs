use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::GetUserAgentResponse;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_agent_returns_current_codex_user_agent() -> Result<()> {
    let codex_home = TempDir::new()?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp.send_get_user_agent_request().await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;

    let received: GetUserAgentResponse = to_response(response)?;
    let os_info = os_info::get();
    let os_type = os_info.os_type();
    let os_version = os_info.version();
    let arch = os_info.architecture().unwrap_or("unknown");
    let expected_prefix = format!("codex_cli_rs/0.0.0 ({os_type} {os_version}; {arch})");

    assert!(
        received.user_agent.starts_with(&expected_prefix),
        "unexpected user agent prefix: {}",
        received.user_agent
    );
    assert!(
        received
            .user_agent
            .ends_with(" (codex-app-server-tests; 0.1.0)"),
        "unexpected user agent suffix: {}",
        received.user_agent
    );
    Ok(())
}
