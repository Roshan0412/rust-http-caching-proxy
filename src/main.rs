use clap::Parser;

mod proxy;
mod cache;
mod utils;

#[derive(Parser, Debug)]
#[command(name = "caching-proxy")]
#[command(about = "A simple caching proxy server", long_about = None)]
struct Args {
    /// Port to run the proxy server on
    #[arg(long)]
    port: Option<u16>,

    /// Origin server URL to forward requests to
    #[arg(long)]
    origin: Option<String>,

    /// Clear the cache and exit
    #[arg(long, default_value_t = false)]
    clear_cache: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.clear_cache {
        cache::clear_cache().await;
        println!("Cache cleared successfully.");
        return;
    }

    let port = args.port.expect("Please specify --port");
    let origin = args.origin.expect("Please specify --origin");

    println!("Starting caching proxy server on port {} forwarding to {}", port, origin);

    proxy::run_proxy(port, origin).await.unwrap();
}
