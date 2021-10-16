use crate::{task::Task, v8};
use std::collections::HashMap;
use std::time::Duration;
use tokio::select;
use tokio::sync::watch;

pub type TimerCancelReceiver = watch::Receiver<bool>;
pub type TimerCancelSender = watch::Sender<bool>;

struct TimerStorage {
    pub index: usize,
    pub store: HashMap<usize, TimerCancelSender>,
}

impl Drop for TimerStorage {
    fn drop(&mut self) {
        for (_, cancel) in self.store.drain() {
            let _ = cancel.send(true);
        }
    }
}

impl TimerStorage {
    pub fn new() -> Self {
        Self {
            index: 0,
            store: HashMap::new(),
        }
    }

    pub fn create(&mut self) -> (usize, TimerCancelReceiver) {
        self.index += 1;

        let (tx, rx) = watch::channel(false);
        self.store.insert(self.index, tx);

        (self.index, rx)
    }

    pub fn cancel(&mut self, id: &usize) {
        if let Some(cancel) = self.store.remove(id) {
            let _ = cancel.send(true);
        }
    }
}

fn clear_timer_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _ret: v8::ReturnValue,
) {
    let id = { args.get(0).integer_value(scope).unwrap() as usize };
    let storage = scope
        .get_slot_mut::<TimerStorage>()
        .expect("Could not get timer storage");

    storage.cancel(&id);
}

fn set_timeout_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let task = SetTimerTask::build(scope, &args, false);
    let queue = scope
        .get_slot::<crate::task::EventLoop>()
        .expect("Could not find the event loop")
        .queue();

    let id = task.id as f64;
    let id = v8::Number::new(scope, id);

    queue.send(Box::new(task));
    ret.set(id.into());
}

fn set_interval_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let task = SetTimerTask::build(scope, &args, true);
    let queue = scope
        .get_slot::<crate::task::EventLoop>()
        .expect("Could not find the event loop")
        .queue();

    let id = task.id as f64;
    let id = v8::Number::new(scope, id);

    queue.send(Box::new(task));
    ret.set(id.into());
}

pub struct ExecuteTimerTask {
    pub id: usize,
    pub callback: v8::Global<v8::Function>,
}

impl Task for ExecuteTimerTask {
    fn execute(&mut self, runtime: &crate::runtime::Runtime) -> std::io::Result<()> {
        let isolate = runtime.isolate();
        let storage = isolate
            .get_slot::<TimerStorage>()
            .expect("Could not get timer storage");

        if !storage.store.contains_key(&self.id) {
            return Ok(());
        }

        let mut scope = runtime.scope();
        let undefined = v8::undefined(&mut scope);
        let callback = self.callback.get(&mut scope);
        callback.call(&mut scope, undefined.into(), &[]);

        Ok(())
    }
}

pub struct SetTimerTask {
    pub id: usize,
    pub repeat: bool,
    pub callback: v8::Global<v8::Function>,
    pub milliseconds: usize,
    pub cancel: TimerCancelReceiver,
}

impl SetTimerTask {
    pub fn build(
        scope: &mut v8::HandleScope,
        args: &v8::FunctionCallbackArguments,
        repeat: bool,
    ) -> Self {
        let (id, cancel) = {
            let storage = scope
                .get_slot_mut::<TimerStorage>()
                .expect("Could not get timer storage");
            storage.create()
        };

        let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
        let callback = v8::Global::new(scope, callback);
        let milliseconds = args.get(1).int32_value(scope).unwrap() as usize;

        SetTimerTask {
            id,
            repeat,
            milliseconds,
            callback,
            cancel,
        }
    }
}

impl Task for SetTimerTask {
    fn execute(&mut self, runtime: &crate::runtime::Runtime) -> std::io::Result<()> {
        let milliseconds = self.milliseconds as u64;
        let repeat = self.repeat;
        let callback = self.callback.clone();
        let queue = runtime.queue();
        let mut cancel = self.cancel.clone();
        let id = self.id;

        runtime.spawn(async move {
            let duration = Duration::from_millis(milliseconds);

            loop {
                select! {
                    _ = cancel.changed() => {
                        if *cancel.borrow() {break};
                    }
                    _ = tokio::time::sleep(duration) => {
                        queue.send(Box::new(ExecuteTimerTask { id, callback: callback.clone() }));
                        if !repeat {break};
                    }
                }
            }
        });

        Ok(())
    }
}

pub fn install(runtime: &crate::runtime::Runtime) {
    let isolate = runtime.isolate();
    isolate.set_slot(TimerStorage::new());

    let mut scope = runtime.scope();

    {
        let key = v8::String::new(&mut scope, "setTimeout").unwrap();
        let value = v8::Function::new(&mut scope, set_timeout_callback).unwrap();

        scope
            .get_current_context()
            .global(&mut scope)
            .set(&mut scope, key.into(), value.into());
    }

    {
        let key = v8::String::new(&mut scope, "setInterval").unwrap();
        let value = v8::Function::new(&mut scope, set_interval_callback).unwrap();

        scope
            .get_current_context()
            .global(&mut scope)
            .set(&mut scope, key.into(), value.into());
    }

    {
        let key = v8::String::new(&mut scope, "clearTimeout").unwrap();
        let value = v8::Function::new(&mut scope, clear_timer_callback).unwrap();

        scope
            .get_current_context()
            .global(&mut scope)
            .set(&mut scope, key.into(), value.into());
    }

    {
        let key = v8::String::new(&mut scope, "clearInterval").unwrap();
        let value = v8::Function::new(&mut scope, clear_timer_callback).unwrap();

        scope
            .get_current_context()
            .global(&mut scope)
            .set(&mut scope, key.into(), value.into());
    }
}
