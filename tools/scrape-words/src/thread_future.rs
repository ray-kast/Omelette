use futures::{prelude::*, task};
use std::{
  sync::{Arc, Barrier, Mutex},
  thread,
};

struct Shared<I, E>
where
  I: Send + 'static,
  E: Send + 'static,
{
  result: Option<Result<I, E>>,
  awaiter: Option<task::Task>,
  bar: Arc<Barrier>,
}

type RcShared<I, E> = Arc<Mutex<Shared<I, E>>>;

pub struct ThreadFuture<I, E>
where
  I: Send + 'static,
  E: Send + 'static,
{
  shared: RcShared<I, E>,
}

impl<I, E> ThreadFuture<I, E>
where
  I: Send + 'static,
  E: Send + 'static,
{
  pub fn new<F>(f: F) -> Self
  where
    F: FnOnce() -> Result<I, E> + Send + 'static,
  {
    let shared = Arc::new(Mutex::new(Shared {
      result: None,
      awaiter: None,
      bar: Arc::new(Barrier::new(2)),
    }));

    let shared_1 = shared.clone();

    // TODO: use an FnBox to lazily spawn the thread
    thread::spawn(move || {
      let shared = shared_1;

      {
        let shared = shared.lock().unwrap();
        shared.bar.clone()
      }.wait();

      let ret = f();

      {
        let mut shared = shared.lock().unwrap();

        shared.result = Some(ret);

        if let Some(ref awaiter) = shared.awaiter {
          awaiter.notify();
        }
      }
    });

    Self { shared }
  }
}

impl<I, E> Future for ThreadFuture<I, E>
where
  I: Send + 'static,
  E: Send + 'static,
{
  type Item = I;
  type Error = E;

  fn poll(&mut self) -> Poll<I, E> {
    if let Some(bar) = {
      let mut shared = self.shared.lock().unwrap();

      match shared.awaiter {
        None => {
          shared.awaiter = Some(task::current());
          Some(shared.bar.clone())
        },
        _ => None
      }
    } {
      bar.wait();
    }

    {
      let mut shared = self.shared.lock().unwrap();

      match shared.result.take() {
        None => Ok(Async::NotReady),
        Some(r) => Ok(Async::Ready(r?)),
      }
    }
  }
}
