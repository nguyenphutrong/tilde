use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use super::*;

#[test]
fn requests_do_not_add_cloud_trace_headers() {
    let client = Client::new();
    for url in [
        "http://127.0.0.1:8080/completion",
        "https://api.github.com/repos/nguyenphutrong/tilde/releases/latest",
    ] {
        let request = client.get(url).build().unwrap();
        assert_eq!(request.wrapped.url().as_str(), url);
        for header in ["x-warp-traceparent", "traceparent", "tracestate", "baggage"] {
            assert!(!request.wrapped.headers().contains_key(header));
        }
    }
}

#[test]
fn loopback_http_still_executes_without_cloud_trace_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nlocal")
            .unwrap();
        String::from_utf8(request).unwrap()
    });
    let response = futures::executor::block_on(async {
        Client::new()
            .get(format!("http://{address}/completion"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    assert_eq!(response, "local");
    let request = server.join().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("get /completion http/1.1\r\n"));
    for header in ["x-warp-traceparent", "traceparent", "tracestate", "baggage"] {
        assert!(!request.contains(&format!("\r\n{header}:")));
    }
}
