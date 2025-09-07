use dashmap::DashMap;
use once_cell::sync::Lazy;

use reqwest::header::HeaderMap;

/// Struct to hold cached response data
pub struct CachedResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

// Global cache: key is String (method + URL), value is CachedResponse
static CACHE: Lazy<DashMap<String, CachedResponse>> = Lazy::new(DashMap::new);

/// Retrieve cached response if exists
pub async fn get_cached_response(key: &str) -> Option<CachedResponse> {
    CACHE.get(key).map(|entry| CachedResponse {
        status: entry.status,
        headers: clone_headers(&entry.headers),
        body: entry.body.clone(),
    })
}

/// Store response in cache
pub async fn store_response(key: &str, status: u16, headers: &HeaderMap, body: &[u8]) {
    // Clone headers but remove hop-by-hop headers before caching
    let mut headers_to_store = HeaderMap::new();
    for (name, value) in headers.iter() {
        if !is_hop_by_hop_header(name.as_str()) {
            headers_to_store.insert(name, value.clone());
        }
    }

    let cached = CachedResponse {
        status,
        headers: headers_to_store,
        body: body.to_vec(),
    };
    CACHE.insert(key.to_string(), cached);
}

/// Clear the entire cache
pub async fn clear_cache() {
    CACHE.clear();
}

/// Helper to clone headers (wrap's HeaderMap doesn't implement Clone)
fn clone_headers(headers: &HeaderMap) -> HeaderMap {
    let mut new_headers = HeaderMap::new();
    for (k, v) in headers.iter() {
        new_headers.insert(k, v.clone());
    }
    new_headers
}

/// Hop-by-hop headers as per RFC 2616 Section 13.5.1 (same as proxy.rs)
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
