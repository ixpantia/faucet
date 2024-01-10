use hyper::body::Body;

pub(crate) trait TSBody: Body + Send + Sync + 'static {}
impl<T: Body + Send + Sync + 'static> TSBody for T {}
