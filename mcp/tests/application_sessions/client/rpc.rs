//! Purpose: Require valid JSON-RPC frames and decode normal MCP tool content.

use super::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

impl Client {
    pub(in super::super) async fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.sequence += 1;
        let id = self.sequence;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await;
        tokio::time::timeout(Duration::from_secs(15), self.receive(id))
            .await
            .expect("MCP response deadline")
    }

    pub(super) async fn send(&mut self, frame: Value) {
        let input = self.input.as_mut().expect("MCP request pipe");
        input
            .write_all(format!("{frame}\n").as_bytes())
            .await
            .expect("write MCP request");
        input.flush().await.expect("flush MCP request");
    }

    async fn receive(&mut self, id: u64) -> Value {
        loop {
            let mut line = String::new();
            assert!(
                self.output
                    .read_line(&mut line)
                    .await
                    .expect("read MCP stdout")
                    > 0,
                "MCP closed stdout before its response"
            );
            assert!(line.len() <= 8 * 1024 * 1024, "MCP frame exceeds its bound");
            let frame: Value = serde_json::from_str(&line)
                .expect("MCP stdout must contain only JSON protocol frames");
            assert_eq!(frame["jsonrpc"], "2.0");
            if frame["id"] == id {
                return frame;
            }
        }
    }

    pub(in super::super) async fn result(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let response = self
            .rpc("tools/call", json!({"name":name,"arguments":arguments}))
            .await;
        if let Some(error) = response.get("error") {
            return Err(error["message"]
                .as_str()
                .expect("MCP error message")
                .to_owned());
        }
        let result = &response["result"];
        let text = result["content"]
            .as_array()
            .expect("tool content")
            .iter()
            .find_map(|part| part["text"].as_str())
            .expect("text content");
        if result["isError"] == true {
            return Err(text.to_owned());
        }
        Ok(serde_json::from_str(text).expect("tool result is structured JSON"))
    }

    pub(in super::super) async fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.result(name, arguments)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }
}
