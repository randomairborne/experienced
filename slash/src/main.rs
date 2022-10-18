mod discord_sig_validation;

#[tokio::main]
async fn main() {
    serve(([0, 0, 0, 0], 8080)).await;
}

pub async fn serve(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tcp_listener = TcpListener::bind(addr).await?;
    loop {
        let (tcp_stream, _) = tcp_listener.accept().await?;
        tokio::task::spawn(async move {
            if let Err(http_err) = Http::new()
                .http1_only(true)
                .http1_keep_alive(true)
                .serve_connection(tcp_stream, service_fn(Self::handle_discord_message))
                .await
            {
                eprintln!("Error while serving HTTP connection: {}", http_err);
            }
        });
    }
}

async fn handle_discord_message(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World!")))
}

// mee6 algorithm: 5 * (lvl ^ 2) + (50 * lvl) + 100 - xp;
