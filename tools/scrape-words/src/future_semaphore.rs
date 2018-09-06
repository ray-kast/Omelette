use futures::{prelude::*, task};
use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

struct Awaiter {
  task: task::Task,
  signalled: Mutex<bool>, // TODO: this doesn't need to be a mutex
}

type RcAwaiter = Arc<Awaiter>;

struct Shared {
  count: usize,
  avail: usize,
  awaiters: VecDeque<RcAwaiter>,
}

type RcShared = Arc<Mutex<Shared>>;

#[derive(Clone)]
pub struct FutureSemaphore {
  shared: RcShared,
}

impl FutureSemaphore {
  pub fn new(n: usize) -> Self {
    Self {
      shared: Arc::new(Mutex::new(Shared {
        count: n,
        avail: n,
        awaiters: VecDeque::new(),
      })),
    }
  }

  pub fn enter(&self) -> EnterFutureSemaphore {
    EnterFutureSemaphore {
      shared: self.shared.clone(),
      awaiter: None,
    }
  }
}

pub struct EnterFutureSemaphore {
  shared: RcShared,
  awaiter: Option<RcAwaiter>,
}

impl Future for EnterFutureSemaphore {
  type Item = FutureSemaphoreGuard;
  type Error = ();

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    let mut shared = self.shared.lock().unwrap();

    if shared.avail > 0 {
      shared.avail = shared.avail - 1;

      Ok(Async::Ready(FutureSemaphoreGuard {
        shared: self.shared.clone(),
      }))
    } else {
      match self.awaiter {
        None => {
          let awaiter = Arc::new(Awaiter {
            task: task::current(),
            signalled: Mutex::new(false),
          });
          self.awaiter = Some(awaiter.clone());

          shared.awaiters.push_back(awaiter);

          Ok(Async::NotReady)
        }
        Some(ref a) => Ok(if *a.signalled.lock().unwrap() {
          Async::Ready(FutureSemaphoreGuard {
            shared: self.shared.clone(),
          })
        } else {
          Async::NotReady
        }),
      }
    }
  }
}

pub struct FutureSemaphoreGuard {
  shared: RcShared,
}

impl Drop for FutureSemaphoreGuard {
  fn drop(&mut self) {
    let mut shared = self.shared.lock().unwrap();

    match shared.awaiters.pop_front() {
      None => {
        shared.avail = shared.avail + 1;
        if shared.avail > shared.count {
          panic!("FutureSemaphore over-released");
        }
      }
      Some(ref a) => {
        *a.signalled.lock().unwrap() = true;
        a.task.notify();
      }
    }
  }
}
