
use hyper::{Body, Request };
use hyper::Uri;

/// Takes a request and builds a new uri to send to the worker.
/// The uri is built by taking the path and query string from the request
/// and appending it to the worker's url.
fn build_uri(base_uri: &Uri, req: &Request<Body>) -> Uri {
    let mut uri = base_uri.clone().into_parts();
    uri.path_and_query = req.uri().path_and_query().cloned();
    Uri::from_parts(uri).expect("failed to build uri")
}

/// Converts a reqwest::Response into an actix_web::HttpResponse
/// This is done by copying the status code and headers from the response
/// and then reading the body into a byte array.
//pub async fn convert_response(res: Response<Body>) -> Response<Body> {
//    let mut res_new = Response::builder()
//        .status(res.status());
//    for (header_name, header_value) in res.headers() {
//        res_new = res_new.header(header_name, header_value);
//    }
//    res_new.body(res.into_body()).expect("failed to build response")
//}

/// Converts an actix_web::HttpRequest into a reqwest::Request
/// This is done by copying the method and uri from the request
/// and then building a new reqwest::Request.
pub fn convert_request(
    base_uri: &Uri,
    req: Request<Body>,
) -> Request<Body> {
    let mut req_new = Request::builder()
        .method(req.method())
        .uri(build_uri(base_uri, &req));
    for (header_name, header_value) in req.headers() {
        req_new = req_new.header(header_name, header_value);
    }
    req_new.body(req.into_body()).expect("failed to build request")
}
