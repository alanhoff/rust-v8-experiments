use crate::v8;

fn log_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let mut result = String::new();

    for i in 0..args.length() {
        let arg = args.get(i);

        if i > 0 {
            result.push(' ');
        }

        result.push_str(&arg.to_rust_string_lossy(scope));
    }

    println!("{}", result);
}

pub fn install(runtime: &crate::runtime::Runtime) {
    let mut scope = runtime.scope();
    let key = v8::String::new(&mut scope, "log").unwrap();
    let value = v8::Function::new(&mut scope, log_callback).unwrap();

    let console = v8::Object::new(&mut scope);
    console.set(&mut scope, key.into(), value.into());

    let key = v8::String::new(&mut scope, "console").unwrap();
    scope
        .get_current_context()
        .global(&mut scope)
        .set(&mut scope, key.into(), console.into());
}
