use std::rc::Rc;
use std::collections::HashMap;
use futures::Future;
use route_recognizer;
use state::Container;

use request::Method;
use super::{Handler, Request, Response, Error};
pub use route_recognizer::Params;


#[derive(Clone)]
pub struct Router {
    rr: Rc<HashMap<Method, route_recognizer::Router<Box<Handler + 'static>>>>,
}

impl Router {
    pub fn new() -> Router {
        Router { rr: Rc::new(HashMap::new()) }
    }

    pub fn get(&mut self, route: &str, handler: Box<Handler + 'static>) {
        let mut rr = Rc::get_mut(&mut self.rr).expect("can't modify router at this point");
        rr.entry(Method::Get)
            .or_insert_with(route_recognizer::Router::new)
            .add(route, handler);
    }

    pub fn mount(&mut self, prefix: &str, handler: Box<Handler + 'static>) {
        let mut rr = Rc::get_mut(&mut self.rr).expect("can't modify router at this point");
        let prefix = if prefix == "/" { "" } else { prefix };

        let path = format!("{}{}", prefix, handler.path());

        rr.entry(handler.method())
            .or_insert_with(route_recognizer::Router::new)
            .add(&path, handler);
    }

    pub fn run(
        &self,
        req: Request,
        state: Container,
    ) -> Result<Box<Future<Item = Response, Error = Error>>, Error> {
        let rr = self.rr.get(req.method()).ok_or(Error::RouterError)?;

        let m = rr.recognize(req.path()).map_err(|_| Error::RouterError)?;
        Ok(m.handler.handle(req, m.params, state))
    }
}