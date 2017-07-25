use std::io::Error as IoError;
use std::net::SocketAddr;

use tokio_core::reactor::{Handle, Core};
use tokio_core::net::TcpListener;

use futures::{Future, Stream, IntoFuture};

use hyper::server::{Http, Request, Response, Service};
use hyper;

use state::Container;

use responder::Responder;
use router::Router;

pub struct Server {
    listener: TcpListener,
    router: Router,
    state: Container,
}

struct S {
    router: Router,
    state: Container,
}

impl Service for S {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        self.serve(req)
    }
}


impl Server {
    pub fn bind(addr: SocketAddr, router: Router, handle: &Handle) -> Server {
        let listener = TcpListener::bind(&addr, handle).expect("unable to listen");
        let state = Container::new();
        Server {
            listener,
            router,
            state,
        }
    }

    pub fn manage_state<T: 'static>(mut self, state: T) -> Server {
        if !self.state.set(state) {
            panic!("double state set");
        }

        self
    }

    pub fn run(self, handle: Handle) -> impl Future<Item = (), Error = IoError> {
        let http = Http::new();
        let router = self.router;
        let state = self.state;

        let service_factory = move || {
            S {
                router: router.clone(),
                state: state.clone(),
            }
        };


        self.listener.incoming().for_each(move |(socket, addr)| {
            http.bind_connection(&handle, socket, addr, service_factory());
            Ok(())
        })
    }

    pub fn start_sync(addr: SocketAddr, router: Router) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let s = Server::bind(addr, router, &handle);

        core.run(s.run(handle)).unwrap();
    }

    pub fn start_sync_with_state<T: 'static>(addr: SocketAddr, router: Router, state: T) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let s = Server::bind(addr, router, &handle).manage_state(state);
        core.run(s.run(handle)).unwrap();
    }
}

impl S {
    fn serve(&self, hreq: Request) -> Box<Future<Item = Response, Error = hyper::Error>> {
        use super::Request as RRequest;

        let f = self.router
            .run(RRequest::new(hreq), self.state.clone())
            .into_future()
            .flatten();

        let f = f.then(|r| match r {
            Ok(r) => Ok(r),
            Err(e) => Ok(e.respond()),
        });

        Box::new(f)
    }
}