use crate::mcp;

pub enum LLMResponse {
    FinalResponse(String),
    ToolCallDetected(mcp::ToolCall),
}

#[derive(serde::Deserialize, Clone)]
pub struct Model {
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct TagsResponse {
    pub models: Vec<Model>,
}

pub async fn ping_ollama(url: String) -> bool {
    let client = reqwest::Client::new();
    let res = client.get(url).send().await;
    res.is_ok()
}

pub async fn fetch_models(url: String) -> Result<Vec<Model>, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client.get(format!("{}/api/tags", url)).send().await?;

    let res_json: TagsResponse = res.json().await?;
    Ok(res_json.models)
}

pub async fn chat_stream(
    messages: Vec<String>,
    model: String,
    url: String,
    system_message: Option<String>,
) -> Result<LLMResponse, reqwest::Error> {
    let client = reqwest::Client::new();

    // Construct the messages for the Ollama API
    let mut ollama_messages = Vec::new();

    // Prepend the system message if it exists
    if let Some(sys_msg) = system_message {
        ollama_messages.push(serde_json::json!({"role": "system", "content": sys_msg}));
    }

    // Add previous messages from the conversation history
    for msg in messages {
        if msg.starts_with("You: ") {
            ollama_messages.push(
                serde_json::json!({"role": "user", "content": msg.strip_prefix("You: ").unwrap()}),
            );
        } else if msg.starts_with("Lucius: ") {
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg.strip_prefix("Lucius: ").unwrap()}));
        } else if msg.starts_with("Tool Result: ") {
            ollama_messages.push(serde_json::json!({"role": "tool", "content": msg.strip_prefix("Tool Result: ").unwrap()}));
        } else if msg.starts_with("Tool Call: ") {
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg}));
        }
    }

    // Construct the request body with the messages
    let req_body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": ollama_messages,
    });

    let mut res = client
        .post(format!("{}/api/chat", url))
        .json(&req_body)
        .send()
        .await?;

    let mut full_response = String::new();
    while let Ok(Some(chunk)) = res.chunk().await {
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(chat_res) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(message) = chat_res["message"].as_object() {
                    if let Some(content) = message["content"].as_str() {
                        full_response.push_str(content);
                        if let Some(tool_call) = mcp::parse_tool_call(&full_response) {
                            return Ok(LLMResponse::ToolCallDetected(tool_call));
                        }
                    }
                }
                if chat_res["done"].as_bool().unwrap_or(false) {
                    return Ok(LLMResponse::FinalResponse(full_response));
                }
            } else {
                log::error!("Failed to parse stream chunk from /api/chat: {}", line);
            }
        }
    }
    if let Some(tool_call) = mcp::parse_tool_call(&full_response) {
        Ok(LLMResponse::ToolCallDetected(tool_call))
    } else {
        Ok(LLMResponse::FinalResponse(full_response))
    }
}

pub async fn handle_llm_turn(
    mcp_request_tx: Option<tokio::sync::mpsc::Sender<mcp::McpRequest>>,
    current_history: Vec<String>,
    model: String,
    url: String,
    lucius_context: Option<String>,
    response_tx: tokio::sync::mpsc::Sender<String>,
) {
    let mut messages_for_llm = current_history.clone();

    loop {
        match chat_stream(
            messages_for_llm.clone(),
            model.clone(),
            url.clone(),
            lucius_context.clone(),
        )
        .await
        {
            Ok(llm_response) => {
                match llm_response {
                    LLMResponse::FinalResponse(response_text) => {
                        if let Err(e) = response_tx.send(response_text).await {
                            log::error!("Failed to send final LLM response to main thread: {}", e);
                        }
                        break;
                    }
                    LLMResponse::ToolCallDetected(tool_call) => {
                        log::info!("Tool Call Detected: {:?}", tool_call);
                        messages_for_llm.push(format!(
                            "Tool Call: {}",
                            serde_json::to_string(&tool_call)
                                .unwrap_or_else(|_| "Invalid tool call format".to_string())
                        ));

                        if let Some(mcp_tx) = &mcp_request_tx {
                            let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                            let mcp_req = mcp::McpRequest {
                                method: tool_call.tool,
                                params: tool_call.params,
                                response_tx: oneshot_tx,
                            };

                            if let Err(e) = mcp_tx.send(mcp_req).await {
                                log::error!("Failed to send MCP request: {}", e);
                                let tool_error_str =
                                    format!("Tool Error: Failed to send MCP request: {}", e);
                                messages_for_llm.push(format!("Tool Result: {}", tool_error_str));
                                continue;
                            }

                            match oneshot_rx.await {
                                Ok(tool_result_or_err) => match tool_result_or_err {
                                    Ok(tool_result) => {
                                        let tool_result_str =
                                            format!("{}", tool_result.to_string());
                                        log::info!("Tool Result: {}", tool_result_str);
                                        messages_for_llm
                                            .push(format!("Tool Result: {}", tool_result_str));
                                    }
                                    Err(e) => {
                                        let tool_error_str = format!("Tool Error: {}", e);
                                        log::error!("{}", tool_error_str);
                                        messages_for_llm
                                            .push(format!("Tool Result: {}", tool_error_str));
                                    }
                                },
                                Err(e) => {
                                    let receive_error_str = format!("Tool Error: Failed to receive response from MCP manager: {}", e);
                                    log::error!("{}", receive_error_str);
                                    messages_for_llm
                                        .push(format!("Tool Result: {}", receive_error_str));
                                }
                            }
                        } else {
                            let no_mcp_msg = "Tool Call detected, but MCP client is not running.";
                            log::error!("{}", no_mcp_msg);
                            messages_for_llm.push(format!("Tool Result: {}", no_mcp_msg));
                        }
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Error from chat stream: {}", e);
                log::error!("{}", err_msg);
                if let Err(send_err) = response_tx.send(err_msg).await {
                    log::error!(
                        "Failed to send chat stream error to main thread: {}",
                        send_err
                    );
                }
                break;
            }
        }
    }
}
