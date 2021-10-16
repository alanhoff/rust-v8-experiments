use std::process::exit;

use js::runtime::Runtime;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new();
    js::ext::cli::install(&runtime);

    runtime.run().await?;
    exit(0);

    Ok(())
}
