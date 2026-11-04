#[cfg(test)]
mod tests {
    use rmcp::model::CallToolRequestParams;
    use rmcp::{ClientHandler, ServiceExt};

    use crate::mcp::server::RidgelineMcp;

    #[derive(Debug, Clone, Default)]
    struct TestClient;

    impl ClientHandler for TestClient {
        fn get_info(&self) -> rmcp::model::ClientInfo {
            rmcp::model::ClientInfo::default()
        }
    }

    fn make_server() -> RidgelineMcp {
        RidgelineMcp::new(vec![])
    }

    #[tokio::test]
    async fn server_initializes_with_correct_info() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let info = client.peer_info().unwrap();

        assert_eq!(info.server_info.name.as_str(), "ridgeline");
        assert_eq!(info.server_info.version.as_str(), env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn lists_all_tools() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();

        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(tool_names.iter().any(|n| n == "list_providers"));
        assert!(tool_names.iter().any(|n| n == "list_pull_requests"));
        assert!(tool_names.iter().any(|n| n == "get_pull_request_detail"));
        assert_eq!(tools.len(), 3);

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn list_providers_returns_empty_array() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let result = client
            .call_tool(CallToolRequestParams::new("list_providers"))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));

        let text = result
            .content
            .first()
            .and_then(|c| c.raw.as_text())
            .map(|t| t.text.as_str())
            .expect("expected text content");

        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed, serde_json::json!([]));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn list_pull_requests_returns_empty_with_no_providers() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let result = client
            .call_tool(CallToolRequestParams::new("list_pull_requests"))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));

        let text = result
            .content
            .first()
            .and_then(|c| c.raw.as_text())
            .map(|t| t.text.as_str())
            .expect("expected text content");

        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["reviewing"], serde_json::json!([]));
        assert_eq!(parsed["authored"], serde_json::json!([]));
        assert_eq!(parsed["errors"], serde_json::json!([]));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn list_pull_requests_with_unknown_provider_filter_returns_error() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let result = client
            .call_tool(
                CallToolRequestParams::new("list_pull_requests").with_arguments(
                    serde_json::json!({ "provider": "nonexistent" })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            )
            .await
            .unwrap();

        assert!(result.is_error.unwrap_or(false));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn get_pr_detail_fails_with_unknown_provider() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let result = client
            .call_tool(
                CallToolRequestParams::new("get_pull_request_detail").with_arguments(
                    serde_json::json!({
                        "provider": "nonexistent",
                        "project": "proj",
                        "repository": "repo",
                        "number": 1
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await
            .unwrap();

        assert!(result.is_error.unwrap_or(false));

        let text = result
            .content
            .first()
            .and_then(|c| c.raw.as_text())
            .map(|t| t.text.as_str())
            .expect("expected error text");

        assert!(text.contains("nonexistent"));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn get_pr_detail_schema_has_required_fields() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();

        let detail_tool = tools
            .iter()
            .find(|t| t.name.to_string() == "get_pull_request_detail")
            .expect("tool should exist");

        let required = detail_tool
            .input_schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("should have required fields");

        let required_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required_names.contains(&"provider"));
        assert!(required_names.contains(&"project"));
        assert!(required_names.contains(&"repository"));
        assert!(required_names.contains(&"number"));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn list_pull_requests_schema_has_optional_filters() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server = make_server();
        let server_handle = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();

        let list_tool = tools
            .iter()
            .find(|t| t.name.to_string() == "list_pull_requests")
            .expect("tool should exist");

        let props = list_tool
            .input_schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("should have properties");

        assert!(props.contains_key("provider"));
        assert!(props.contains_key("project"));

        // provider and project should NOT be required (they're Optional)
        let required = list_tool
            .input_schema
            .get("required")
            .and_then(|v| v.as_array());
        let required_names: Vec<&str> = required
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        assert!(!required_names.contains(&"provider"));
        assert!(!required_names.contains(&"project"));

        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }
}
