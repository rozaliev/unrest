use futures::{Future, Stream};
use {Request, Error};

pub trait FromData: Sized {
    fn from_data(&[u8]) -> Result<Self, Error>;
}

pub fn from_data_req<T: FromData>(req: Request) -> impl Future<Item = T, Error = Error> {
    req.body().concat2().map_err(|e| e.into()).and_then(
        |chunk| {
            FromData::from_data(&*chunk)
        },
    )

}