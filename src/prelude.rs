use bytes::Bytes;

/// Takes a request and builds a new uri to send to the worker.
/// The uri is built by taking the path and query string from the request
/// and appending it to the worker's url.
fn build_uri(base_url: &url::Url, req: &actix_web::HttpRequest) -> url::Url {
    let mut url = base_url.clone();
    url.set_path(req.path());
    url.set_query(Some(req.query_string()));
    url
}

/// Converts a reqwest::Response into an actix_web::HttpResponse
/// This is done by copying the status code and headers from the response
/// and then reading the body into a byte array.
pub async fn convert_response(res: reqwest::Response) -> actix_web::HttpResponse {
    let mut builder = actix_web::HttpResponseBuilder::new(res.status());
    // We copy every header from the response into the builder
    for (key, value) in res.headers() {
        builder.append_header((key, value));
    }
    // We read the body into a byte array and then set the body of the builder
    builder.body(res.bytes().await.expect("failed to read body into bytes"))
}

/// Converts an actix_web::HttpRequest into a reqwest::Request
/// This is done by copying the method and uri from the request
/// and then building a new reqwest::Request.
pub fn convert_request(
    client: &reqwest::Client,
    base_url: &url::Url,
    req: actix_web::HttpRequest,
    payload: Bytes,
) -> reqwest::Request {
    client
        .request(req.method().clone(), build_uri(base_url, &req))
        .body(payload)
        .build()
        .expect("failed to build request")
}
