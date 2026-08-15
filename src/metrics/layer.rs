use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::http::Request;
use axum::response::Response;
use tower::{Layer, Service};

use crate::metrics::Metrics;

#[derive(Clone)]
pub(crate) struct MetricsLayer {
    metrics: Arc<Metrics>,
}

impl MetricsLayer {
    pub(crate) fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService {
            inner,
            metrics: Arc::clone(&self.metrics),
        }
    }
}

#[derive(Clone)]
pub(crate) struct MetricsService<S> {
    inner: S,
    metrics: Arc<Metrics>,
}

impl<S, B> Service<Request<B>> for MetricsService<S>
where
    S: Service<Request<B>, Response = Response, Error = Infallible>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = MetricsFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        self.metrics.begin_request();
        MetricsFuture {
            inner: self.inner.call(request),
            metrics: Arc::clone(&self.metrics),
            started: Instant::now(),
            completed: false,
        }
    }
}

pub(crate) struct MetricsFuture<F> {
    inner: F,
    metrics: Arc<Metrics>,
    started: Instant,
    completed: bool,
}

impl<F> Future for MetricsFuture<F>
where
    F: Future<Output = Result<Response, Infallible>>,
{
    type Output = Result<Response, Infallible>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        let poll = inner.poll(cx);
        if let Poll::Ready(Ok(response)) = &poll {
            this.completed = true;
            this.metrics
                .record_request(response.status(), this.started.elapsed());
        }
        poll
    }
}

impl<F> Drop for MetricsFuture<F> {
    fn drop(&mut self) {
        if !self.completed {
            self.metrics.cancel_request();
        }
    }
}
