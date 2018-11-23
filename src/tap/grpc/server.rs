use bytes::IntoBuf;
use futures::{future, sync::mpsc, Poll, Stream};
use http::HeaderMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tower_grpc::{self as grpc, Response};
use tower_h2::Body as Payload;

use api::tap as api;

use super::match_::Match;
use proxy::http::HasH2Reason;
use tap::{self, Inspect, Subscribe};

// Buffer ~10 req/rsp pairs' worth of events.
const PER_REQUEST_BUFFER_CAPACITY: usize = 40;

#[derive(Clone, Debug)]
pub struct Server<I, T> {
    inspect: I,
    subscribe: T,
}

#[derive(Debug)]
pub struct ResponseStream<I> {
    rx: mpsc::Receiver<api::TapEvent>,
    tap: Arc<Tap<I>>,
}

#[derive(Clone, Debug)]
pub struct Tap<I> {
    tx: mpsc::Sender<api::TapEvent>,
    match_: Match,
    remaining: Arc<AtomicUsize>,
    inspect: I,
}

pub struct TapRequestBody(mpsc::Sender<api::TapEvent>);

pub struct TapResponse(mpsc::Sender<api::TapEvent>);

pub struct TapResponseBody(mpsc::Sender<api::TapEvent>);

impl<I: Inspect, T: Subscribe<Tap<I>>> Server<I, T> {
    pub(in tap) fn new(inspect: I, subscribe: T) -> Self {
        Self { inspect, subscribe }
    }
}

fn invalid_arg(msg: http::header::HeaderValue) -> grpc::Error {
    let status = grpc::Status::with_code(grpc::Code::InvalidArgument);
    let mut headers = HeaderMap::new();
    headers.insert("grpc-message", msg);
    grpc::Error::Grpc(status, headers)
}

impl<I, T> api::server::Tap for Server<I, T>
where
    I: Inspect + Clone,
    T: Subscribe<Tap<I>> + Clone
{
    type ObserveStream = ResponseStream<I>;
    type ObserveFuture = future::FutureResult<Response<Self::ObserveStream>, grpc::Error>;

    fn observe(&mut self, req: grpc::Request<api::ObserveRequest>) -> Self::ObserveFuture {
        let req = req.into_inner();

        if req.limit == 0 {
            let v = http::header::HeaderValue::from_static("limit must be positive");
            return future::err(invalid_arg(v));
        }

        let match_ = match Match::try_new(req.match_) {
            Ok(m) => m,
            Err(e) => {
                let v = format!("{}", e)
                    .parse()
                    .or_else(|_| "invalid message".parse())
                    .unwrap();
                return future::err(invalid_arg(v));
            }
        };

        let (tx, rx) = mpsc::channel(PER_REQUEST_BUFFER_CAPACITY);
        let tap = Arc::new(Tap {
            tx,
            match_,
            inspect: self.inspect.clone(),
            remaining: Arc::new((req.limit as usize).into()),
        });
        self.subscribe.subscribe(Arc::downgrade(&tap));

        future::ok(Response::new(ResponseStream { rx, tap }))
    }
}

impl<I: Inspect> Stream for ResponseStream<I> {
    type Item = api::TapEvent;
    type Error = grpc::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.rx.poll().or_else(|_| Ok(None.into()))
    }
}

impl<I: Inspect> tap::Tap for Tap<I> {
    type TapRequestBody = TapRequestBody;
    type TapResponse = TapResponse;
    type TapResponseBody = TapResponseBody;

    fn tap<B: Payload>(&self, req: &http::Request<B>) -> Option<(TapRequestBody, TapResponse)> {
        if !self.match_.matches(&req, &self.inspect) {
            return None;
        }

        fn req_open<B>(req: &http::Request<B>) -> api::TapEvent {
            unimplemented!()
        }

        let msg = req_open(req);

        loop {
            // Determine whether a tap should be emitted by decrementing `remaining`.
            //
            // `AtomicUsize::fetch_sub` cannot be used because it wraps on overflow.
            let n = self.remaining.load(Ordering::Acquire);

            // If there are no more requests to tap, drop the sender so that the
            // receiver closes immediately.
            if n == 0 {
                return None;
            }

            // Claim a tap slot by decrementing the remaining count.
            let m = self.remaining.compare_and_swap(n, n - 1, Ordering::AcqRel);
            if n != m {
                // Another task claimed the slot, so try again.
                continue;
            }

            // If the receiver event isn't actually written to the channel,
            // return None so that we don't do work for an unaccounted request.
            let mut tx = self.tx.clone();
            return tx.try_send(msg).ok().map(move |()| {
                let req = TapRequestBody(tx.clone());
                let rsp = TapResponse(tx);
                (req, rsp)
            });
        }
    }
}

impl tap::TapResponse for TapResponse {
    type TapBody = TapResponseBody;

    fn tap<B: Payload>(self, req: &http::Response<B>) -> TapResponseBody {
        fn rsp_open<B>(req: &http::Response<B>) -> api::TapEvent {
            unimplemented!()
        }

        let _ = self.0.try_send(rsp_open(req));
        TapResponseBody(self.0)
    }

    fn fail<E: HasH2Reason>(self, _: &E) {
        unimplemented!()
    }
}

impl tap::TapBody for TapRequestBody {
    fn data<B: IntoBuf>(&mut self, _: &B::Buf) {
        unimplemented!()
    }

    fn end(self, _: Option<&http::HeaderMap>) {
        unimplemented!()
    }

    fn fail(self, _: &h2::Error) {
        unimplemented!()
    }
}

impl tap::TapBody for TapResponseBody {
    fn data<B: IntoBuf>(&mut self, _: &B::Buf) {
        unimplemented!()
    }

    fn end(self, _: Option<&http::HeaderMap>) {
        unimplemented!()
    }

    fn fail(self, _: &h2::Error) {
        unimplemented!()
    }
}