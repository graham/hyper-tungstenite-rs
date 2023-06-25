use bytes::Bytes;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use hyper_tungstenite::tungstenite::Error;
use std::net::Ipv6Addr;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{Message, Result};

use assert2::{assert, let_assert};
use tokio::net::TcpListener;

#[tokio::test]
async fn hyper() {
    // Bind a TCP listener to an ephemeral port.
    let_assert!(Ok(listener) = TcpListener::bind((Ipv6Addr::LOCALHOST, 0u16)).await);
    let_assert!(Ok(bind_addr) = listener.local_addr());

    tokio::task::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();

        tokio::task::spawn(async move {
            http1::Builder::new()
                .serve_connection(stream, service_fn(upgrade_websocket))
                .await
        });
    });
    // Try to create a websocket connection with the server.
    let_assert!(Ok(stream) = TcpStream::connect(bind_addr).await);
    let_assert!(Ok((mut stream, _response)) = tokio_tungstenite::client_async("ws://localhost/foo", stream).await);

    let_assert!(Some(Ok(message)) = stream.next().await);
    assert!(message == Message::text("Hello!"));

    let_assert!(Ok(()) = stream.send(Message::text("Goodbye!")).await);
    assert!(let Some(Ok(Message::Close(None))) = stream.next().await);
}

async fn upgrade_websocket(mut request: Request<IncomingBody>) -> Result<Response<Full<Bytes>>> {
    assert!(hyper_tungstenite::is_upgrade_request(&request) == true);

    let (response, stream) =
        hyper_tungstenite::upgrade(&mut request, None).map_err(Error::Protocol)?;
    tokio::spawn(async move {
        let_assert!(Ok(mut stream) = stream.await);
        assert!(let Ok(()) = stream.send(Message::text("Hello!")).await);
        let_assert!(Some(Ok(reply)) = stream.next().await);
        assert!(reply == Message::text("Goodbye!"));
        assert!(let Ok(()) = stream.send(Message::Close(None)).await);
    });

    Ok(response)
}
