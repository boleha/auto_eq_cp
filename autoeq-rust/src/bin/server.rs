use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let router = autoeq::server::create_router();

    println!("AutoEq REST API server listening on http://{}", addr);
    println!("  POST /equalize   — full pipeline (EQ + PEQ + Graphic EQ)");
    println!("  POST /peq        — parametric EQ optimization only");
    println!("  POST /graphic-eq — graphic EQ string only");
    println!("  GET  /configs    — list available PEQ configurations");
    println!("  GET  /health     — health check");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}