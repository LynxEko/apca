// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use url::Url;

use tokio::net::TcpStream;

use tracing::debug;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_futures::Instrument;

use tungstenite::connect_async;
use tungstenite::MaybeTlsStream;
use tungstenite::WebSocketStream;

use websocket_util::wrap::Wrapper;

use crate::Error;


/// A custom [`Result`]-style type that we can implement a foreign trait
/// on.
#[derive(Debug)]
#[doc(hidden)]
pub enum MessageResult<T, E> {
  /// The success value.
  Ok(T),
  /// The error value.
  Err(E),
}

impl<T, E> From<Result<T, E>> for MessageResult<T, E> {
  #[inline]
  fn from(result: Result<T, E>) -> Self {
    match result {
      Ok(t) => Self::Ok(t),
      Err(e) => Self::Err(e),
    }
  }
}


/// Internal function to connect to websocket server.
async fn connect_internal(url: &Url) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Error> {
  let span = span!(Level::DEBUG, "stream");

  async move {
    debug!(message = "connecting", url = display(url));

    // We just ignore the response & headers that are sent along after
    // the connection is made. Alpaca does not seem to be using them,
    // really.
    let (stream, response) = connect_async(url).await?;
    debug!("connection successful");
    trace!(response = debug(&response));

    Ok(stream)
  }
  .instrument(span)
  .await
}


/// Connect to websocket server.
pub async fn connect(
  url: &Url,
) -> Result<Wrapper<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error> {
  connect_internal(url)
    .await
    .map(|stream| Wrapper::builder().build(stream))
}


#[cfg(test)]
pub mod test {
  use super::*;

  use std::future::Future;

  use websocket_util::test::mock_server;
  use websocket_util::test::WebSocketStream;
  use websocket_util::tungstenite::Error as WebSocketError;

  use crate::subscribable::Subscribable;
  use crate::ApiInfo;


  /// The fake key-id we use.
  pub const KEY_ID: &str = "USER12345678";
  /// The fake secret we use.
  pub const SECRET: &str = "justletmein";


  /// Instantiate a dummy websocket server serving messages as per the
  /// provided function `f` and attempt to connect to it to stream
  /// messages.
  pub async fn mock_stream<S, F, R>(f: F) -> Result<(S::Stream, S::Subscription), Error>
  where
    S: Subscribable<Input = ApiInfo>,
    F: FnOnce(WebSocketStream) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), WebSocketError>> + Send + Sync + 'static,
  {
    let addr = mock_server(f).await;
    let api_info = ApiInfo {
      base_url: Url::parse(&format!("ws://{}", addr.to_string())).unwrap(),
      key_id: KEY_ID.to_string(),
      secret: SECRET.to_string(),
    };

    S::connect(&api_info).await
  }
}
