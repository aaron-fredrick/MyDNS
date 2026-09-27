use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use axum::extract::ws::Message;
use futures_util::{Sink, Stream};
use tokio::sync::broadcast;

use crate::mydns::web::ws::handle_socket_stream;

#[derive(Clone, Default)]
struct TestSink {
    messages: Arc<Mutex<Vec<Message>>>,
    fail: bool,
}

impl Sink<Message> for TestSink {
    type Error = std::io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.fail {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "sink failed",
            )))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        if self.fail {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "sink failed",
            ))
        } else {
            self.messages.lock().unwrap().push(item);
            Ok(())
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

struct TestStream {
    items: VecDeque<Result<Message, axum::Error>>,
}

impl TestStream {
    fn new(items: Vec<Result<Message, axum::Error>>) -> Self {
        Self {
            items: items.into(),
        }
    }
}

impl Stream for TestStream {
    type Item = Result<Message, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.items.pop_front())
    }
}

#[tokio::test]
async fn ws_forwards_broadcast_logs_to_client() {
    let (log_tx, mut log_rx) = broadcast::channel(16);
    let sink = TestSink::default();
    let sent_messages = Arc::clone(&sink.messages);
    let stream = futures_util::stream::pending::<Result<Message, axum::Error>>();

    let handle = tokio::spawn(async move {
        handle_socket_stream(sink, stream, &mut log_rx).await;
    });

    log_tx
        .send("[DNS] query home.arpa from 127.0.0.1".into())
        .unwrap();
    log_tx
        .send("[DNS] query example.com from 127.0.0.1".into())
        .unwrap();

    // Give the task a brief moment to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Terminate the broadcast channel
    drop(log_tx);
    let _ = handle.await;

    let msgs = sent_messages.lock().unwrap();
    assert_eq!(msgs.len(), 2);
    match &msgs[0] {
        Message::Text(text) => assert_eq!(text, "[DNS] query home.arpa from 127.0.0.1"),
        other => panic!("expected text message, got {other:?}"),
    }
    match &msgs[1] {
        Message::Text(text) => assert_eq!(text, "[DNS] query example.com from 127.0.0.1"),
        other => panic!("expected text message, got {other:?}"),
    }
}

#[tokio::test]
async fn ws_terminates_on_client_close_message() {
    let (_log_tx, mut log_rx) = broadcast::channel(16);
    let sink = TestSink::default();
    let stream = TestStream::new(vec![
        Ok(Message::Ping(vec![1, 2, 3])),
        Ok(Message::Close(None)),
    ]);

    let handle = tokio::spawn(async move {
        handle_socket_stream(sink, stream, &mut log_rx).await;
    });

    let res = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    assert!(res.is_ok(), "handler should terminate on Close frame");
}

#[tokio::test]
async fn ws_terminates_when_receiver_ends() {
    let (_log_tx, mut log_rx) = broadcast::channel(16);
    let sink = TestSink::default();
    let stream = TestStream::new(vec![]);

    let handle = tokio::spawn(async move {
        handle_socket_stream(sink, stream, &mut log_rx).await;
    });

    let res = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    assert!(res.is_ok(), "handler should terminate when receiver ends");
}

#[tokio::test]
async fn ws_terminates_on_broadcast_channel_close() {
    let (log_tx, mut log_rx) = broadcast::channel(16);
    let sink = TestSink::default();
    let stream = futures_util::stream::pending::<Result<Message, axum::Error>>();

    let handle = tokio::spawn(async move {
        handle_socket_stream(sink, stream, &mut log_rx).await;
    });

    drop(log_tx);

    let res = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    assert!(
        res.is_ok(),
        "handler should terminate when broadcast channel closes"
    );
}

#[tokio::test]
async fn ws_terminates_when_sink_fails() {
    let (log_tx, mut log_rx) = broadcast::channel(16);
    let sink = TestSink {
        messages: Arc::new(Mutex::new(Vec::new())),
        fail: true,
    };
    let stream = futures_util::stream::pending::<Result<Message, axum::Error>>();

    let handle = tokio::spawn(async move {
        handle_socket_stream(sink, stream, &mut log_rx).await;
    });

    log_tx
        .send("[DNS] query home.arpa from 127.0.0.1".into())
        .unwrap();

    let res = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    assert!(
        res.is_ok(),
        "handler should terminate when sender sink fails"
    );
}
