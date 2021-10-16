use crate::runtime::Runtime;
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;
use std::{cell::RefCell, rc::Rc};

pub type SyncMessage = Box<dyn FnOnce(&Runtime)>;
pub type AsyncMessage = Box<dyn FnOnce(&Runtime) -> Pin<Box<dyn Future<Output = ()>>>>;

pub struct Channel {
    pub tx: mpsc::UnboundedSender<AsyncMessage>,
    pub rx: Rc<RefCell<mpsc::UnboundedReceiver<AsyncMessage>>>,
}

impl Default for Channel {
  fn default() -> Self {
      Self::new()
  }
}

impl Channel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self { tx, rx: Rc::new(RefCell::new(rx)) }
    }

    pub fn sync_sender(&self) -> Box<dyn Fn(SyncMessage)> {
      let tx = self.tx.clone();

      Box::new(move |func| {
        let _ = tx.send(Box::new(move |runtime| {
          func(runtime);
          Box::pin(async {})
        })).is_ok();
      })
    }

    pub fn sender(&self) -> Box<dyn Fn(AsyncMessage)> {
      let tx = self.tx.clone();

      Box::new(move |func| {
        let _ = tx.send(Box::new(move |runtime| {
          func(runtime)
        })).is_ok();
      })
    }
}
