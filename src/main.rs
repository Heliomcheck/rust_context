use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use anyhow::{Context, Result}; 

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    let listner = TcpListener::bind(args[1].as_str()).await
        .context("Can't bind to address")?;

    let (mut socket, _) = listner.accept().await
        .context("Can't accept connection")?;
        
    
    let mut buff = String::new();
    socket.read_to_string(&mut buff).await
        .context("Can't read data from socket")?;
    println!("{}", buff);
    
    Ok(())
}
