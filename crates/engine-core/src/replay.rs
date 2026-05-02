use std::sync::Arc;

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;

use crate::http_message::{HttpSession, Scheme};
use crate::mitm::build_tls_client_config;

pub async fn replay_session(session: &HttpSession) -> Result<(u16, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    let req = &session.request;
    let url = req.url.as_deref().unwrap_or("");
    let method = req.method.as_deref().unwrap_or("GET");
    let is_https = req.scheme == Scheme::Https;

    let uri: hyper::Uri = url.parse()?;
    let host = uri.host().ok_or("No host in URL")?.to_string();
    let port = uri.port_u16().unwrap_or(if is_https { 443 } else { 80 });
    let body_bytes = req.body.clone().unwrap_or_default();

    if is_https {
        replay_https(&host, port, method, &uri, &req.headers, &body_bytes).await
    } else {
        replay_http(&host, port, method, &uri, &req.headers, &body_bytes).await
    }
}

async fn replay_http(
    host: &str,
    port: u16,
    method: &str,
    uri: &hyper::Uri,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u16, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("Replay HTTP connection error: {}", e);
        }
    });

    let mut builder = Request::builder().method(method).uri(uri);
    for (key, value) in headers {
        let kl = key.to_lowercase();
        if kl != "host" && kl != "connection" && kl != "proxy-connection" && kl != "proxy-authorization" {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }
    builder = builder.header("Host", host);
    builder = builder.header("Connection", "close");

    let request = builder.body(Full::new(Bytes::from(body.to_vec())))?;
    let response = sender.send_request(request).await?;

    let status = response.status().as_u16();
    let resp_body = response.into_body().collect().await.map(|b| b.to_bytes().to_vec())?;

    Ok((status, resp_body))
}

async fn replay_https(
    host: &str,
    port: u16,
    method: &str,
    uri: &hyper::Uri,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(u16, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    let stream = tokio::net::TcpStream::connect((host, port)).await?;

    let client_config = build_tls_client_config()?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;
    let tls_stream = connector.connect(server_name, stream).await?;

    let io = TokioIo::new(tls_stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("Replay HTTPS connection error: {}", e);
        }
    });

    let mut builder = Request::builder().method(method).uri(uri);
    for (key, value) in headers {
        let kl = key.to_lowercase();
        if kl != "host" && kl != "connection" && kl != "proxy-connection" && kl != "proxy-authorization" && kl != "accept-encoding" {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }
    builder = builder.header("Host", host);
    builder = builder.header("Connection", "close");
    builder = builder.header("Accept-Encoding", "identity");

    let request = builder.body(Full::new(Bytes::from(body.to_vec())))?;
    let response = sender.send_request(request).await?;

    let status = response.status().as_u16();
    let resp_body = response.into_body().collect().await.map(|b| b.to_bytes().to_vec())?;

    Ok((status, resp_body))
}
