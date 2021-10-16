use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

async fn read_stdin() -> std::io::Result<String> {
    let stdin = tokio::io::stdin();
    let mut line = String::new();
    let mut reader = BufReader::new(stdin);

    reader.read_line(&mut line).await?;

    Ok(line)
}

async fn write_stdout(out: &str) -> std::io::Result<()> {
    let mut stdout = tokio::io::stdout();
    stdout.write_all(out.as_bytes()).await?;
    stdout.flush().await?;

    Ok(())
}

pub struct CliEvalTask {
    pub script: String,
    pub exit: bool
}

impl crate::task::Task for CliEvalTask {
    fn stop(&mut self) -> bool {
        self.exit
    }

    fn execute(&mut self, runtime: &crate::runtime::Runtime) -> std::io::Result<()> {
        if self.exit {
            return Ok(());
        }

        if let Some(value) = runtime.eval(&self.script) {
            let mut scope = runtime.scope();
            let result = value.get(&mut scope).to_rust_string_lossy(&mut scope);

            runtime.spawn(async move {
                write_stdout(&format!("{}\n> ", result)).await?;

                std::io::Result::Ok(())
            });
        }

        Ok(())
    }
}

pub fn install(runtime: &crate::runtime::Runtime) {
    let queue = runtime.queue();

    runtime.spawn(async move {
        write_stdout("Welcome to AlanJS!\n> ").await?;

        while let Ok(script) = read_stdin().await {
            let exit = &script == "exit\n";

            let _ = queue.send(Box::new(CliEvalTask {script, exit}));
        }

        std::io::Result::Ok(())
    });
}
