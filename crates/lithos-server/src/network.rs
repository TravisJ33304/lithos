//! WebSocket network layer — accepts connections and bridges to the game loop.

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use lithos_protocol::{ClientMessage, EntityId, codec};

/// Events sent from network tasks to the game loop.
#[derive(Debug)]
pub enum NetworkEvent {
    /// A new client connected.
    Connected {
        entity_id: EntityId,
        outbound_tx: mpsc::UnboundedSender<Bytes>,
    },
    /// A client sent a message.
    Message {
        entity_id: EntityId,
        message: ClientMessage,
    },
    /// A client disconnected.
    Disconnected { entity_id: EntityId },
}

/// Handle a single WebSocket connection.
///
/// Spawned as a tokio task for each accepted TCP connection.
pub async fn handle_connection(
    stream: TcpStream,
    entity_id: EntityId,
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
) {
    let addr = stream.peer_addr().ok();
    tracing::info!(?addr, entity_id = entity_id.0, "new TCP connection");

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!(?addr, "WebSocket handshake failed: {e}");
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Channel for outbound messages (game loop → this client).
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<Bytes>();

    // Notify game loop of new connection.
    let _ = event_tx.send(NetworkEvent::Connected {
        entity_id,
        outbound_tx,
    });

    // Spawn a task to forward outbound messages to the WebSocket.
    let write_handle = tokio::spawn(async move {
        while let Some(bytes) = outbound_rx.recv().await {
            if ws_tx.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // Read loop: forward inbound messages to the game loop.
    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => match codec::decode::<ClientMessage>(&data) {
                Ok(client_msg) => {
                    let _ = event_tx.send(NetworkEvent::Message {
                        entity_id,
                        message: client_msg,
                    });
                }
                Err(e) => {
                    tracing::warn!(entity_id = entity_id.0, "decode error: {e}");
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => {} // Ignore text, ping, pong frames.
            Err(e) => {
                tracing::warn!(entity_id = entity_id.0, "ws read error: {e}");
                break;
            }
        }
    }

    // Client disconnected.
    let _ = event_tx.send(NetworkEvent::Disconnected { entity_id });
    write_handle.abort();
    tracing::info!(entity_id = entity_id.0, "connection closed");
}
