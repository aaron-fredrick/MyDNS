use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::state::AppState;

/// `GET /ws`
///
/// Upgrades the connection to a WebSocket and streams every log event that
/// passes through the broadcast channel.  Clients receive newline-delimited
/// plain-text log strings in real time.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.log_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    loop {
        tokio::select! {
            // Forward broadcast log events to the WebSocket client.
            msg = rx.recv() => {
                match msg {
                    Ok(log_line) => {
                        if sender.send(Message::Text(log_line)).await.is_err() {
                            break;
                        }
                    }
                    // Lagged or channel closed.
                    Err(_) => break,
                }
            }

            // Handle incoming frames (mainly Close).
            frame = receiver.next() => {
                match frame {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // ignore ping/pong/text/binary from client
                }
            }
        }
    }
}
