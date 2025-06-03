use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use reqwest::Client;
use std::convert::Infallible;
use std::net::SocketAddr;
use bytes::Bytes;

// Helper: Validate content type
fn is_json_content_type(req: &Request<Body>) -> bool {
    req.headers().get("content-type") == Some(&hyper::header::HeaderValue::from_static("application/json"))
}

// Helper: Read body
async fn read_body(req: Request<Body>) -> Result<bytes::Bytes, Response<Body>> {
    match hyper::body::to_bytes(req.into_body()).await {
        Ok(b) => Ok(b),
        Err(_) => Err(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Failed to read body"))
            .unwrap()),
    }
}

// Helper: Parse JSON as Value (accepts any fields)
fn parse_json(body: &[u8]) -> Result<serde_json::Value, Response<Body>> {
    match serde_json::from_slice(body) {
        Ok(v) => Ok(v),
        Err(_) => Err(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid JSON format"))
            .unwrap()),
    }
}

// Helper: Forward to external API
async fn forward_to_external_api(client: &Client, data: &serde_json::Value) -> Result<Response<Body>, Response<Body>> {
    match client.post("https://postman-echo.com/post")
        .json(data)
        .send()
        .await
    {
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Ok(json_resp) => {
                    let body = match serde_json::to_string_pretty(&json_resp) {
                        Ok(s) => s,
                        Err(_) => "Failed to serialize response".to_string(),
                    };
                    Ok(Response::new(Body::from(body)))
                }
                Err(_) => Err(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from("Failed to decode API response"))
                    .unwrap()),
            }
        }
        Err(e) => Err(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("API request failed: {}", e)))
            .unwrap()),
    }
}

async fn handle_request(req: Request<Body>, client: Client) -> Result<Response<Body>, Infallible> {
    if req.method() == Method::POST && req.uri().path() == "/hello" {
        if !is_json_content_type(&req) {
            return Ok(Response::builder()
                .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
                .body(Body::from("Expected application/json"))
                .unwrap());
        }

        // Read body
        let body = match read_body(req).await {
            Ok(b) => b,
            Err(resp) => return Ok(resp),
        };

        // Parse JSON (accept any fields)
        let data = match parse_json(&body) {
            Ok(v) => v,
            Err(resp) => return Ok(resp),
        };

        // Forward to external API
        match forward_to_external_api(&client, &data).await {
            Ok(resp) => Ok(resp),
            Err(resp) => Ok(resp),
        }
    } else {
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap())
    }
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    // HTTPS client (for outgoing requests only)
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .expect("Failed to build reqwest client");

    let make_svc = make_service_fn(move |_| {
        let client = client.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, client.clone())
            }))
        }
    });
    println!("Listening on http://{}", addr);

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}