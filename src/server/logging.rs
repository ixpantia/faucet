use hyper::{http::HeaderValue, Method, Request, Response, Uri, Version};

use super::onion::{Layer, Service};
use crate::server::service::State;
use std::{net::IpAddr, time};

trait StateLogData: Send + Sync + 'static {
    fn get_state_data(&self) -> (IpAddr, &'static str);
}

impl StateLogData for State {
    #[inline(always)]
    fn get_state_data(&self) -> (IpAddr, &'static str) {
        let ip = self.remote_addr;
        let target = self.client.config.target;
        (ip, target)
    }
}

#[derive(PartialEq, Eq)]
enum LogOption<T> {
    None,
    Some(T),
}

impl<T> From<Option<T>> for LogOption<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => LogOption::None,
            Some(v) => LogOption::Some(v),
        }
    }
}

impl<T> std::fmt::Display for LogOption<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, "-"),
            LogOption::Some(v) => write!(f, "{}", v),
        }
    }
}

impl<T> std::fmt::Debug for LogOption<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, r#""-""#),
            LogOption::Some(v) => write!(f, "{:?}", v),
        }
    }
}

struct LogData {
    target: &'static str,
    ip: IpAddr,
    method: Method,
    path: Uri,
    version: Version,
    status: u16,
    user_agent: LogOption<HeaderValue>,
    elapsed: u128,
}

impl LogData {
    fn log(&self) {
        log::info!(
            target: self.target,
            r#"{ip} "{method} {path} {version:?}" {status} {user_agent:?} {elapsed}"#,
            ip = self.ip,
            method = self.method,
            path = self.path,
            version = self.version,
            status = self.status,
            user_agent = self.user_agent,
            elapsed = self.elapsed,
        );
    }
}

#[inline(always)]
async fn capture_log_data<Body, ResBody, Error, State: StateLogData>(
    inner: &impl Service<Request<Body>, Response = Response<ResBody>, Error = Error>,
    req: Request<Body>,
) -> Result<(Response<ResBody>, LogData), Error> {
    let start = time::Instant::now();

    // Extract request info for logging
    let state = req.extensions().get::<State>().expect("State not found");
    let (ip, target) = state.get_state_data();
    let method = req.method().clone();
    let path = req.uri().clone();
    let version = req.version();
    let user_agent: LogOption<_> = req.headers().get(hyper::header::USER_AGENT).cloned().into();

    // Make the request
    let res = inner.call(req, None).await?;

    // Extract response info for logging
    let status = res.status().as_u16();
    let elapsed = start.elapsed().as_millis();

    let log_data = LogData {
        target,
        ip,
        method,
        path,
        version,
        status,
        user_agent,
        elapsed,
    };

    Ok((res, log_data))
}

pub(super) struct LogService<S> {
    inner: S,
}

impl<S, Body, ResBody> Service<Request<Body>> for LogService<S>
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Send + Sync,
{
    type Error = S::Error;
    type Response = Response<ResBody>;

    async fn call(
        &self,
        req: Request<Body>,
        _: Option<IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        let (res, log_data) = capture_log_data::<_, _, _, State>(&self.inner, req).await?;

        log_data.log();

        Ok(res)
    }
}

pub(super) struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}

#[cfg(test)]
mod tests {
    use hyper::StatusCode;

    use super::*;

    #[tokio::test]
    async fn log_capture() {
        #[derive(Clone)]
        struct MockState;

        impl StateLogData for MockState {
            fn get_state_data(&self) -> (IpAddr, &'static str) {
                (IpAddr::V4([127, 0, 0, 1].into()), "test")
            }
        }

        struct Svc;

        impl Service<Request<()>> for Svc {
            type Response = Response<()>;
            type Error = ();
            async fn call(
                &self,
                _: Request<()>,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                Ok(Response::builder().status(StatusCode::OK).body(()).unwrap())
            }
        }

        let req = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .extension(MockState)
            .version(Version::HTTP_11)
            .header(hyper::header::USER_AGENT, "test")
            .body(())
            .unwrap();

        let (_, log_data) = capture_log_data::<_, _, _, MockState>(&Svc, req)
            .await
            .unwrap();

        assert_eq!(log_data.ip, IpAddr::V4([127, 0, 0, 1].into()));
        assert_eq!(log_data.method, Method::GET);
        assert_eq!(log_data.path, "https://example.com/");
        assert_eq!(log_data.version, Version::HTTP_11);
        assert_eq!(log_data.status, 200);
        assert_eq!(
            log_data.user_agent,
            LogOption::Some(HeaderValue::from_static("test"))
        );
        assert!(log_data.elapsed > 0);
        assert_eq!(log_data.target, "test");
    }

    #[test]
    fn log_option_display() {
        assert_eq!(LogOption::<u8>::None.to_string(), "-");
        assert_eq!(LogOption::Some(1).to_string(), "1");
    }

    #[test]
    fn log_option_debug() {
        assert_eq!(format!("{:?}", LogOption::<u8>::None), r#""-""#);
        assert_eq!(format!("{:?}", LogOption::Some(1)), "1");
    }

    #[test]
    fn log_option_from_option() {
        assert_eq!(LogOption::<u8>::from(None), LogOption::None);
        assert_eq!(LogOption::from(Some(1)), LogOption::Some(1));
    }

    #[test]
    fn log_data_log() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        struct Buffer(Arc<Mutex<Vec<u8>>>);

        impl Write for Buffer {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.0.lock().unwrap().flush()
            }
        }

        impl Buffer {
            fn clone_buf(&self) -> Vec<u8> {
                self.0.lock().unwrap().clone()
            }
        }

        impl Clone for Buffer {
            fn clone(&self) -> Self {
                Buffer(Arc::clone(&self.0))
            }
        }

        let log_data = LogData {
            target: "test",
            ip: IpAddr::V4([127, 0, 0, 1].into()),
            method: Method::GET,
            path: "https://example.com/".parse().unwrap(),
            version: Version::HTTP_11,
            status: 200,
            user_agent: LogOption::Some(HeaderValue::from_static("test")),
            elapsed: 5,
        };

        let buf = Buffer(Arc::new(Mutex::new(Vec::new())));
        let mut logger = env_logger::Builder::new();
        // ALWAYS USE INFO LEVEL FOR LOGGING
        logger.filter_level(log::LevelFilter::Info);
        logger.format(|f, record| writeln!(f, "{}", record.args()));
        logger.target(env_logger::Target::Pipe(Box::new(buf.clone())));
        logger.init();

        log_data.log();

        let log = String::from_utf8(buf.clone_buf()).unwrap();

        assert_eq!(
            log.trim(),
            r#"127.0.0.1 "GET https://example.com/ HTTP/1.1" 200 "test" 5"#
        )
    }
}
