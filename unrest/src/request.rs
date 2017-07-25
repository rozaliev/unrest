use hyper::Request as HyperRequest;

pub use hyper::Method;
pub use hyper::Body;

pub struct Request {
    r: HyperRequest,
}

impl Request {
    pub(crate) fn new(r: HyperRequest) -> Request {
        Request { r }
    }

    pub fn method(&self) -> &Method {
        self.r.method()
    }

    pub fn path(&self) -> &str {
        self.r.path()
    }

    pub fn body(self) -> Body {
        self.r.body()
    }
}
