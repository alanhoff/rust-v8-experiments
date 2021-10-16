use js::runtime::Runtime;


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new();
    runtime.eval(include_str!("../../script.js"));
    runtime.run().await?;

    Ok(())
}
