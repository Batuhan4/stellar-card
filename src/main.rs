use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = stellar_card::cli::Cli::parse();
    std::process::exit(stellar_card::app::run(cli).await);
}
