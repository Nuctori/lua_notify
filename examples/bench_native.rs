//! Native notify benchmark — baseline for the Lua binding overhead.
//! Output format is `name=value` lines consumed by `make bench`.
//! Events are OS-bound; this measures the per-event queue/table cost.

use notify::{RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Instant;

fn main() {
    let dir = std::env::temp_dir().join("lua_notify_bench_native");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("f.txt");

    // native: watch + fire N events + receive via channel
    let (tx, rx) = channel();
    let mut w = notify::recommended_watcher(move |res| {
        if let Ok(ev) = res {
            let _ = tx.send(ev);
        }
    })
    .unwrap();
    w.watch(&dir, RecursiveMode::NonRecursive).unwrap();

    const N: usize = 200;
    let t = Instant::now();
    for i in 0..N {
        std::fs::write(&file, format!("{i}")).unwrap();
        std::fs::remove_file(&file).unwrap();
    }
    let mut received = 0usize;
    while let Ok(_) = rx.try_recv() {
        received += 1;
    }
    // ensure at least a few events arrived
    std::thread::sleep(std::time::Duration::from_millis(100));
    while let Ok(_) = rx.try_recv() {
        received += 1;
    }
    let poll_ns = t.elapsed().as_nanos() as f64 / N as f64;

    println!("poll_ns={poll_ns:.0}");
    println!("events={received}");
    let _ = std::fs::remove_dir_all(&dir);
}
