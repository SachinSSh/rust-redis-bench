use redis::aio::ConnectionManager;

/// Creates a single `ConnectionManager` that auto-reconnects on failure.
///
/// `ConnectionManager` is cheaply cloneable — every clone shares the same
/// underlying multiplexed TCP connection.  This is sufficient for localhost
/// benchmarking; for production you'd front it with a connection pool.
pub async fn connect(url: &str) -> ConnectionManager {
    let client = redis::Client::open(url).unwrap_or_else(|e| {
        eprintln!("❌ Invalid Redis URL \"{url}\": {e}");
        std::process::exit(1);
    });

    ConnectionManager::new(client).await.unwrap_or_else(|e| {
        eprintln!("❌ Cannot connect to Redis: {e}");
        eprintln!("   Make sure redis-server is running on localhost:6379");
        eprintln!("   → brew services start redis");
        eprintln!("   → sudo systemctl start redis");
        eprintln!("   → redis-server");
        std::process::exit(1);
    })
}