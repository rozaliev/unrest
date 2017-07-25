use futures::Future;
use {Request, Response, Params, Error};
use request::Method;
use Container;

pub trait Handler {
    fn handle(&self, Request, Params, Container) -> Box<Future<Item = Response, Error = Error>>;
    fn path(&self) -> &'static str;
    fn method(&self) -> Method;
}