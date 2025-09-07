use warp::http::{HeaderMap as WarpHeaderMap, HeaderName as WarpHeaderName, HeaderValue as WarpHeaderValue};

pub fn normalize_path(path: &str) -> String {
    if path.ends_with('/') && path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path.to_string()
    }
}

pub fn convert_reqwest_headers_to_warp(reqwest_headers: &reqwest::header::HeaderMap) -> WarpHeaderMap {
    let mut warp_headers = WarpHeaderMap::new();

    for (key, value) in reqwest_headers.iter() {
        if let (Ok(k), Ok(v)) = (
            WarpHeaderName::from_bytes(key.as_str().as_bytes()),
            WarpHeaderValue::from_bytes(value.as_bytes()),
        ) {
            warp_headers.insert(k, v);
        }
    }

    warp_headers
}
