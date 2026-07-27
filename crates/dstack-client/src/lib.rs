//! Dstack TEE guest agent client.

mod client;
mod error;
mod types;

pub use client::DstackClient;
pub use error::DstackError;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_is_in_tee_when_socket_not_exists() {
        let client = DstackClient::new("/nonexistent/socket/path");
        assert!(!client.is_in_tee().await);
    }

    #[tokio::test]
    async fn test_get_app_info_when_socket_not_exists() {
        let client = DstackClient::new("/nonexistent/socket/path");
        let result = client.get_app_info().await;
        assert!(result.is_err());
        assert!(matches!(result, Err(DstackError::SocketNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_quote_when_socket_not_exists() {
        let client = DstackClient::new("/nonexistent/socket/path");
        let result = client.get_quote(b"test data").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(DstackError::SocketNotFound(_))));
    }

    #[tokio::test]
    async fn test_derive_key_when_socket_not_exists() {
        let client = DstackClient::new("/nonexistent/socket/path");
        let result = client.derive_key("/test/path", None).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(DstackError::SocketNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_ra_tls_cert_when_socket_not_exists() {
        let client = DstackClient::new("/nonexistent/socket/path");
        let result = client.get_ra_tls_cert().await;
        assert!(result.is_err());
        assert!(matches!(result, Err(DstackError::SocketNotFound(_))));
    }

    #[test]
    fn test_app_info_deserialization() {
        let json = r#"{
            "app_id": "test-app",
            "compose_hash": "abc123",
            "instance_id": "instance-1",
            "custom_field": "extra"
        }"#;

        let info: AppInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.app_id, Some("test-app".into()));
        assert_eq!(info.compose_hash, Some("abc123".into()));
        assert_eq!(info.instance_id, Some("instance-1".into()));
    }

    #[test]
    fn test_app_info_with_missing_fields() {
        let json = r#"{}"#;

        let info: AppInfo = serde_json::from_str(json).unwrap();
        assert!(info.app_id.is_none());
        assert!(info.compose_hash.is_none());
        assert!(info.instance_id.is_none());
    }

    #[test]
    fn test_quote_deserialization() {
        let json = r#"{
            "quote": "base64encodedquote",
            "report_data": "hexdata"
        }"#;

        let quote: Quote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.quote, "base64encodedquote");
        assert_eq!(quote.report_data, Some("hexdata".into()));
    }

    #[test]
    fn test_derive_key_request_serialization() {
        let request = DeriveKeyRequest {
            path: "/test/path".into(),
            subject: Some("test-subject".into()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"path\":\"/test/path\""));
        assert!(json.contains("\"subject\":\"test-subject\""));
    }

    #[test]
    fn test_derive_key_request_without_subject() {
        let request = DeriveKeyRequest {
            path: "/test/path".into(),
            subject: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"path\":\"/test/path\""));
        assert!(!json.contains("subject"));
    }

    #[test]
    fn test_derive_key_response_deserialization() {
        let json = r#"{"key": "deadbeef"}"#;

        let response: DeriveKeyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.key, "deadbeef");
    }

    #[test]
    fn test_ra_tls_cert_deserialization() {
        let json = r#"{"cert": "Y2VydGlmaWNhdGU="}"#;

        let cert: RaTlsCert = serde_json::from_str(json).unwrap();
        assert_eq!(cert.cert, "Y2VydGlmaWNhdGU=");
    }

    /// Spawn a temporary Unix-socket HTTP server that answers dstack guest-agent routes.
    async fn start_mock_guest_agent() -> (tempfile::TempDir, String, tokio::task::JoinHandle<()>) {
        use hyper::service::{make_service_fn, service_fn};
        use hyper::{Body, Request, Response, Server, StatusCode};
        use hyperlocal::UnixServerExt;
        use std::convert::Infallible;

        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("dstack.sock");
        let sock_str = sock.to_string_lossy().to_string();

        async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
            let path = req.uri().path().to_string();
            let body = match path.as_str() {
                "/Info" => {
                    r#"{"app_id":"app-1","compose_hash":"hash-1","instance_id":"inst-1"}"#.into()
                }
                p if p.starts_with("/GetQuote") => {
                    r#"{"quote":"cXVvdGU=","report_data":"aabb"}"#.into()
                }
                "/DeriveKey" => r#"{"key":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}"#.into(),
                "/GetRaTlsCert" => {
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    let cert = STANDARD.encode(b"certificate-bytes");
                    format!(r#"{{"cert":"{cert}"}}"#)
                }
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("missing"))
                        .unwrap());
                }
            };
            Ok(Response::new(Body::from(body)))
        }

        let make_svc =
            make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
        let server = Server::bind_unix(&sock).unwrap().serve(make_svc);
        let handle = tokio::spawn(async move {
            let _ = server.await;
        });

        // Wait briefly for the socket file to appear.
        for _ in 0..50 {
            if sock.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        (dir, sock_str, handle)
    }

    #[tokio::test]
    async fn happy_path_info_quote_key_and_cert() {
        let (_dir, sock, handle) = start_mock_guest_agent().await;
        let client = DstackClient::new(&sock);

        assert!(client.is_in_tee().await);

        let info = client.get_app_info().await.unwrap();
        assert_eq!(info.app_id.as_deref(), Some("app-1"));
        assert_eq!(info.compose_hash.as_deref(), Some("hash-1"));

        let quote = client.get_quote(b"challenge-bytes").await.unwrap();
        assert_eq!(quote.quote, "cXVvdGU=");

        let key = client.derive_key("/prefs", Some("subj")).await.unwrap();
        assert_eq!(key.len(), 32);

        let cert = client.get_ra_tls_cert().await.unwrap();
        assert_eq!(cert, b"certificate-bytes");

        handle.abort();
    }

    #[tokio::test]
    async fn http_error_from_guest_agent() {
        use hyper::service::{make_service_fn, service_fn};
        use hyper::{Body, Request, Response, Server, StatusCode};
        use hyperlocal::UnixServerExt;
        use std::convert::Infallible;

        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("err.sock");
        let sock_str = sock.to_string_lossy().to_string();

        async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("guest boom"))
                .unwrap())
        }

        let make_svc =
            make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
        let server = Server::bind_unix(&sock).unwrap().serve(make_svc);
        let handle = tokio::spawn(async move {
            let _ = server.await;
        });
        for _ in 0..50 {
            if sock.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let client = DstackClient::new(sock_str);
        let err = client.get_app_info().await.unwrap_err();
        assert!(matches!(err, DstackError::QuoteGeneration(_)));
        handle.abort();
    }

    #[tokio::test]
    async fn derive_key_rejects_invalid_hex() {
        use hyper::service::{make_service_fn, service_fn};
        use hyper::{Body, Request, Response, Server};
        use hyperlocal::UnixServerExt;
        use std::convert::Infallible;

        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("badhex.sock");
        let sock_str = sock.to_string_lossy().to_string();

        async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
            Ok(Response::new(Body::from(r#"{"key":"not-hex"}"#)))
        }

        let make_svc =
            make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
        let server = Server::bind_unix(&sock).unwrap().serve(make_svc);
        let handle = tokio::spawn(async move {
            let _ = server.await;
        });
        for _ in 0..50 {
            if sock.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let client = DstackClient::new(sock_str);
        let err = client.derive_key("/x", None).await.unwrap_err();
        assert!(matches!(err, DstackError::KeyDerivation(_)));
        handle.abort();
    }
}
