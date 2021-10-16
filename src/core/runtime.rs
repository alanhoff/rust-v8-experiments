use crate::{task, v8};
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

static V8_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct Runtime {
    pub isolate: *mut v8::OwnedIsolate,
    pub global: v8::Global<v8::Context>,
    pub executor: Rc<tokio::task::LocalSet>,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.isolate) };
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn init() {
        if !V8_INITIALIZED.swap(true, Ordering::SeqCst) {
            v8::V8::set_flags_from_string(
                "--predictable --gc-global --use-strict --single-threaded --optimize-for-size --always-compact --expose-gc",
            );

            let platform = v8::new_single_threaded_default_platform(false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        }
    }

    pub fn new() -> Self {
        Self::init();

        let (isolate, global) = {
            let mut isolate = v8::Isolate::new(v8::CreateParams::default());
            isolate.set_slot(task::EventLoop::new());

            let global = {
                let mut scope = v8::HandleScope::new(&mut isolate);
                let context = v8::Context::new(&mut scope);
                v8::Global::new(&mut scope, context)
            };

            (isolate, global)
        };

        let runtime = Self {
            isolate: Box::into_raw(Box::new(isolate)),
            global,
            executor: Rc::new(tokio::task::LocalSet::new()),
        };

        // Install extensions
        crate::ext::console::install(&runtime);
        crate::ext::timers::install(&runtime);

        runtime
    }

    #[allow(clippy::mut_from_ref)]
    pub fn isolate(&self) -> &mut v8::OwnedIsolate {
        unsafe { Box::leak(Box::from_raw(self.isolate)) }
    }

    pub fn gc(&self) {
        let isolate = self.isolate();

        isolate.low_memory_notification();
        isolate.clear_kept_objects();
        isolate.perform_microtask_checkpoint();

        self.pump();
    }

    pub fn pump(&self) {
        let platform = v8::V8::get_current_platform();
        let isolate = self.isolate();

        loop {
            if !v8::Platform::pump_message_loop(&platform, isolate, false) {
                break;
            }
        }
    }

    pub fn scope(&self) -> v8::HandleScope {
        let context = self.global.clone();
        let isolate = self.isolate();

        v8::HandleScope::with_context(isolate, context)
    }

    pub fn eval(&self, script: &str) -> Option<v8::Global<v8::Value>> {
        let mut scope = self.scope();

        let string = v8::String::new(&mut scope, script).unwrap();
        let code = v8::Script::compile(&mut scope, string, None).unwrap();
        let result = code.run(&mut scope).map(|result| v8::Global::new(&mut scope, result));

        self.pump();

        result
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.executor.spawn_local(future)
    }

    pub fn queue(&self) -> task::TaskSender {
        self.isolate()
            .get_slot::<task::EventLoop>()
            .expect("Could not find the event loop")
            .queue()
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let executor = self.executor.clone();
        let event_loop = self
            .isolate()
            .get_slot::<task::EventLoop>()
            .expect("Unable to find the event loop");

        let rx = event_loop.receiver.clone();

        executor
            .run_until(async move {
                let mut rx = rx.lock().await;

                while let Some(mut task) = rx.recv().await {
                    task.execute(self)?;
                    self.pump();

                    if task.stop() {
                        break;
                    }
                }

                Ok(()) as std::io::Result<()>
            })
            .await?;

        Ok(())
    }

    pub fn shutdown() {
        unsafe { v8::V8::dispose() };
        v8::V8::shutdown_platform();
    }
}
