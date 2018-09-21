use std::marker::PhantomData;

#[derive(Debug)]
pub struct Optional<T, N, L> {
    inner: Option<L>,
    _p: PhantomData<fn() -> (T, N)>
}

impl<T, N, L> Optional<T, N, L>
where
    N: super::Make<T>,
    L: super::Layer<T, T, N, Value = N::Value, Error = N::Error>,
{
    pub fn some(layer: L) -> Self {
        Some(layer).into()
    }

    pub fn none() -> Self {
        None.into()
    }

    pub fn when<F: FnOnce() -> L>(predicate: bool, mk: F) -> Self {
        if predicate {
            Self::some(mk())
        } else {
            Self::none()
        }
    }
}

impl<T, N, L> super::Layer<T, T, N> for Optional<T, N, L>
where
    N: super::Make<T>,
    L: super::Layer<T, T, N, Value = N::Value, Error = N::Error>,
{
    type Value = <super::Either<N, L::Make> as super::Make<T>>::Value;
    type Error = <super::Either<N, L::Make> as super::Make<T>>::Error;
    type Make = super::Either<N, L::Make>;

    fn bind(&self, next: N) -> Self::Make {
        match self.inner.as_ref() {
            None => super::Either::A(next),
            Some(ref m) => super::Either::B(m.bind(next)),
        }
    }
}

impl<T, N, L> From<Option<L>> for Optional<T, N, L>
where
    N: super::Make<T>,
    L: super::Layer<T, T, N, Value = N::Value, Error = N::Error>,
{
    fn from(inner: Option<L>) -> Self {
        Optional { inner, _p: PhantomData }
    }
}