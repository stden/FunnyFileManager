//! Purpose: Keep child protocol pipes isolated from MCP stdio and bounded in time.

use super::{protocol, Error, Request};
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::Child;

pub(super) struct Connection {
    pub(super) child: Option<Child>,
    input: Box<dyn AsyncWrite + Send + Unpin>,
    output: BufReader<Box<dyn AsyncRead + Send + Unpin>>,
    exchange: Exchange,
}

enum Exchange {
    Ready,
    InFlight,
}

impl Connection {
    pub(super) fn from_io(
        read: impl AsyncRead + Send + Unpin + 'static,
        write: impl AsyncWrite + Send + Unpin + 'static,
    ) -> Self {
        Self {
            child: None,
            input: Box::new(write),
            output: BufReader::new(Box::new(read)),
            exchange: Exchange::Ready,
        }
    }

    pub(super) async fn handshake(&mut self) -> Result<Value, Error> {
        let value = protocol::result(&protocol::read_frame(&mut self.output).await?)?;
        if value.get("protocol_version").and_then(Value::as_u64) != Some(1) {
            return Err(Error::Protocol(
                "unsupported FFM automation protocol version",
            ));
        }
        value
            .get("state")
            .cloned()
            .ok_or(Error::Protocol("FFM handshake has no state"))
    }

    pub(super) async fn exchange(&mut self, request: &Request) -> Result<Value, Error> {
        if matches!(self.exchange, Exchange::InFlight) {
            return Err(Error::Protocol(
                "previous FFM automation request was interrupted; session closed",
            ));
        }
        let mut frame = serde_json::to_vec(request)
            .map_err(|_serialization| Error::Invalid("could not encode automation request"))?;
        if frame.len() >= protocol::MAX_REQUEST_BYTES {
            return Err(Error::Invalid("automation request exceeds 4 MiB"));
        }
        frame.push(b'\n');
        self.exchange = Exchange::InFlight;
        self.input.write_all(&frame).await?;
        self.input.flush().await?;
        let envelope = protocol::read_frame(&mut self.output).await?;
        self.exchange = Exchange::Ready;
        protocol::result(&envelope)
    }

    pub(super) async fn reap(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if matches!(
            tokio::time::timeout(Duration::from_secs(2), child.wait()).await,
            Ok(Ok(_))
        ) {
            return;
        }
        if let Err(error) = child.kill().await {
            tracing::debug!(%error, "FFM child already stopped while closing");
        }
    }
}
