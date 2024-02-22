use std::sync::Arc;

use config::ServerConfig;
use monoio::{
    io::{AsyncReadRentExt, AsyncWriteRentExt},
    net::TcpListener,
    RuntimeBuilder,
};

fn main() {
    let cfg = Arc::new(ServerConfig::parse());
    println!(
        "Running ping pong server with Monoio.\nPacket size: {}\nListen {}\nCPU slot: {}",
        cfg.byte_count,
        cfg.bind,
        config::format_cores(&cfg.cores)
    );

    let mut threads = Vec::new();
    for cpu in cfg.cores.iter() {
        let cfg_ = cfg.clone();
        let cpu_ = *cpu as _;
        let h = std::thread::spawn(move || {
            monoio::utils::bind_to_cpu_set(Some(cpu_)).unwrap();
            let mut rt = RuntimeBuilder::<monoio::IoUringDriver>::new()
                .with_entries(32768)
                .build()
                .unwrap();
            rt.block_on(serve(cfg_));
        });
        threads.push(h);
    }
    for h in threads {
        let _ = h.join();
    }
}

async fn serve(cfg: Arc<ServerConfig>) {
    let listener = TcpListener::bind(&cfg.bind).unwrap();
    while let Ok((mut stream, _)) = listener.accept().await {
        let byte_count = cfg.byte_count as usize;
        monoio::spawn(async move {
            let mut buf = vec![0; byte_count];
            loop {
                let (r, buf_r) = stream.read_exact(buf).await;
                if r.is_err() {
                    // The connection is closed.
                    return;
                }
                let (w, buf_w) = stream.write_all(buf_r).await;
                if w.is_err() {
                    // The connection is closed.
                    return;
                }
                buf = buf_w;
            }
        });
    }
}
