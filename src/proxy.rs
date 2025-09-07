use std::convert::Infallible;
use warp::{Filter, http::Response, hyper::Body};
use reqwest::Client;
use crate::cache::{get_cached_response, store_response};
use std::sync::Arc;
use crate::utils::{convert_reqwest_headers_to_warp, normalize_path};

use warp::http::{HeaderName, HeaderValue};

pub async fn run_proxy(port: u16, origin: String) -> Result<(), Box<dyn std::error::Error>> {
    // Shared HTTP client for forwarding requests
    let client = Arc::new(Client::new());
    let origin = Arc::new(origin);

    // Clone for warp filters
    let client_filter = warp::any().map(move || client.clone());
    let origin_filter = warp::any().map(move || origin.clone());

    // Catch-all route to proxy all requests
    let proxy_route = warp::any()
        .and(warp::method())
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and(client_filter)
        .and(origin_filter)
        .and_then(proxy_handler);

    println!("Proxy running on http://localhost:{}", port);
    warp::serve(proxy_route).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

async fn proxy_handler(
    method: warp::http::Method,
    path: warp::path::FullPath,
    headers: warp::http::HeaderMap,
    body: bytes::Bytes,
    client: Arc<Client>,
    origin: Arc<String>,
) -> Result<impl warp::Reply, Infallible> {
    let normalized_path = normalize_path(path.as_str());
    let url = format!("{}{}", origin.as_str(), normalized_path);

    // Use method + url as cache key (simple)
    let cache_key = format!("{}:{}", method, url);

    // Check cache
    if let Some(cached_response) = get_cached_response(&cache_key).await {
        let mut resp = Response::builder()
            .status(cached_response.status)
            .body(Body::from(cached_response.body))
            .unwrap();

        let headers = convert_reqwest_headers_to_warp(&cached_response.headers);
        *resp.headers_mut() = headers;
        resp.headers_mut().insert("X-Cache", "HIT".parse().unwrap());

        return Ok(resp);
    }

    // Not cached: forward the request to origin
    let method = reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap();
    let mut req = client.request(method, &url);

    // Copy headers except hop-by-hop headers like host, connection
    for (name, value) in headers.iter() {
        if !is_hop_by_hop_header(name.as_str()) && name != "host" {
            let name = name.as_str();
            let value = value.to_str().unwrap();
            req = req.header(name, value);
        }
    }

    if !body.is_empty() {
        req = req.body(body.clone());
    }

    let resp = req.send().await;

    match resp {
        Ok(resp) => {
            let status = resp.status();
            let headers = resp.headers().clone();
            let bytes = resp.bytes().await.unwrap_or_default();

            // Cache the response (clone headers and body)
            store_response(&cache_key, status.as_u16(), &headers, &bytes).await;

            // Return response with X-Cache: MISS
            let status = warp::http::StatusCode::from_u16(status.as_u16()).unwrap();
            let mut response = Response::builder()
                .status(status)
                .body(Body::from(bytes))
                .unwrap();

            for (k, v) in headers.iter() {
                response.headers_mut().insert(
                    HeaderName::from_bytes(k.as_str().as_bytes()).unwrap(),
                    HeaderValue::from_bytes(v.as_bytes()).unwrap(),
                );
            }
            response.headers_mut().insert("X-Cache", "MISS".parse().unwrap());

            Ok(response)
        }
        Err(_) => {
            // Return 502 Bad Gateway on error
            let resp = Response::builder()
                .status(502)
                .body(Body::from("Bad Gateway"))
                .unwrap();
            Ok(resp)
        }
    }
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "proxy-connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}
