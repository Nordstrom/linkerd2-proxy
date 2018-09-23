use futures::Poll;
use http;
use std::marker::PhantomData;

use svc;

#[derive(Debug)]
pub struct Layer<T>(PhantomData<fn() -> T>);

#[derive(Clone, Debug)]
pub struct Make<M>(M);

#[derive(Clone, Debug)]
pub struct Service<T, S> {
    target: T,
    inner: S,
}

impl<T> Layer<T> {
    pub fn new() -> Self {
        Layer(PhantomData)
    }
}

impl<T> Clone for Layer<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T, M, B> svc::Layer<T, T, M> for Layer<T>
where
    T: Clone + Send + Sync + 'static,
    M: svc::Make<T>,
    M::Value: svc::Service<Request = http::Request<B>>,
{
    type Value = <Make<M> as svc::Make<T>>::Value;
    type Error = <Make<M> as svc::Make<T>>::Error;
    type Make = Make<M>;

    fn bind(&self, next: M) -> Self::Make {
        Make(next)
    }
}

impl<T, M, B> svc::Make<T> for Make<M>
where
    T: Clone + Send + Sync + 'static,
    M: svc::Make<T>,
    M::Value: svc::Service<Request = http::Request<B>>,
{
    type Value = Service<T, M::Value>;
    type Error = M::Error;

    fn make(&self, t: &T) -> Result<Self::Value, Self::Error> {
        let target = t.clone();
        let inner = self.0.make(t)?;
        Ok(Service { inner, target })
    }
}

impl<T, S, B> svc::Service for Service<T, S>
where
    T: Clone + Send + Sync + 'static,
    S: svc::Service<Request = http::Request<B>>,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, mut req: Self::Request) -> Self::Future {
        req.extensions_mut().insert(self.target.clone());
        self.inner.call(req)
    }
}
