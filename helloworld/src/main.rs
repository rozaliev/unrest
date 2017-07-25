#![feature(proc_macro)]
#![feature(conservative_impl_trait)]
#![feature(type_ascription)]

extern crate unrest;
extern crate unrest_codegen;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate hyper;

use futures::future::ok;



use unrest_codegen::handler;
use unrest::{Server, Router, Json};



#[derive(Deserialize, Debug)]
struct SomeInput {
    test: String,
    yo: u64,
}

#[derive(Debug)]
struct SomeState {
    hello: String,
}


#[handler(get("/simple"))]
fn simple() -> impl Future<Item = impl Responder, Error = Error> {
    ok("ok simple")
}

mod nested {
    use unrest_codegen::handler;
    use super::*;

    #[handler(get("/simple_nested"))]
    pub(crate) fn simple_nested() -> impl Future<Item = impl Responder, Error = Error> {
        ok("ok nested")
    }
}


#[handler(get("/with_params/:first/something/:second"))]
fn with_params(first: String, second: bool) -> impl Future<Item = impl Responder, Error = Error> {
    println!("first: {:?}, second: {:?}", first, second);
    ok("ok")
}

#[handler(post("/with_data/:yo/something/:man", data = "input"))]
fn with_data(
    yo: bool,
    man: u32,
    input: Json<SomeInput>,
) -> impl Future<Item = impl Responder, Error = Error> {
    println!("yo: {:?}, man: {:?}, input: {:?}", yo, man, input);
    ok("ok")
}

#[handler(get("/with_state"))]
fn with_state(somestate: State<SomeState>) -> impl Future<Item = impl Responder, Error = Error> {
    println!("somestate: {:?}", somestate);
    ok("ok")
}

#[handler(get("/with_state_and_data", data = "inp"))]
fn with_state_and_data(
    somestate: State<SomeState>,
    inp: Json<SomeInput>,
) -> impl Future<Item = impl Responder, Error = Error> {
    println!("somestate: {:?}, input: {:?}", somestate, inp);
    ok("ok")
}


fn main() {
    let mut router = Router::new();
    router.mount("/", simple());
    router.mount("/", with_params());
    router.mount("/", with_data());
    router.mount("/", nested::simple_nested());
    router.mount("/", with_state());
    router.mount("/", with_state_and_data());



    let addr = "127.0.0.1:3000".parse().unwrap();

    Server::start_sync_with_state(addr, router, SomeState { hello: "yo".to_string() })
}