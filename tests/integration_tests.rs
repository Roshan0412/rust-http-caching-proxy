use std::net::SocketAddr;
use std::time::Duration;
use tokio::task;
use reqwest::Client;

#[tokio::test]
async fn test_cache_hit_and_miss() {
    // Start a dummy origin server
    let origin_addr = start_dummy_origin().await;

    // Start the caching proxy server in a background task
    let proxy_port = 4000;
    let origin_url = format!("http://{}", origin_addr);

    task::spawn(async move {
        caching_proxy::proxy::run_proxy(proxy_port, origin_url).await.unwrap();
    });

    // Wait a bit for the proxy server to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/test", proxy_port);

    // First request - expect MISS
    let resp1 = client.get(&url).send().await.unwrap();
    assert_eq!(resp1.headers().get("X-Cache").unwrap(), "MISS");
    let body1 = resp1.text().await.unwrap();
    assert_eq!(body1, "Hello from origin!");

    // Second request - expect HIT
    let resp2 = client.get(&url).send().await.unwrap();
    assert_eq!(resp2.headers().get("X-Cache").unwrap(), "HIT");
    let body2 = resp2.text().await.unwrap();
    assert_eq!(body2, "Hello from origin!");
}

/// Starts a dummy origin HTTP server responding with fixed text at /test
async fn start_dummy_origin() -> SocketAddr {
    use warp::Filter;

    let route = warp::path("test").map(|| "Hello from origin!");

    // Bind to random port (port 0)
    let (addr, server) = warp::serve(route)
        .bind_ephemeral(([127, 0, 0, 1], 0));

    tokio::spawn(server);

    addr
}
