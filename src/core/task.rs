use crate::runtime::Runtime;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub type SenderHalf = mpsc::UnboundedSender<TaskMessage>;
pub type ReceiverHalf = mpsc::UnboundedReceiver<TaskMessage>;
pub type TaskMessage = Box<dyn Task>;

pub trait Task {
    fn execute(&mut self, runtime: &Runtime) -> std::io::Result<()>;
    fn stop(&mut self) -> bool {
      false
    }
}

pub struct TaskSender {
    sender: SenderHalf
}

impl TaskSender {
    pub fn send(&self, task: TaskMessage) {
        let _ = self.sender.send(task);
    }
}

pub struct EventLoop {
    pub sender: SenderHalf,
    pub receiver: Arc<Mutex<ReceiverHalf>>,
}

impl Default for EventLoop {
    fn default() -> Self {
      Self::new()
    }
}

impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn queue(&self) -> TaskSender {
        let sender = self.sender.clone();

        TaskSender {sender}
    }
}
