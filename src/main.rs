use std::path::PathBuf;

use clap::Parser;
use hickory_server::ServerFuture;
use tokio::{fs, net::UdpSocket};

use crate::{config::Config, handler::CustomRequestHandler};

mod config;
mod forwarder;
mod handler;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    bind: String,
    #[arg(long, short)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = toml::from_str::<Config>(&fs::read_to_string(&args.config).await?)?;

    let socket = UdpSocket::bind(args.bind).await?;
    let handler = CustomRequestHandler::new(config);

    let mut server = ServerFuture::new(handler);
    server.register_socket(socket);
    server.block_until_done().await?;

    Ok(())
}
