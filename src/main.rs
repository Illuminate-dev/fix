use std::error::Error;

use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    fix::cli::cli(std::env::var("OPENAI_API_KEY".to_string())?).await
}
