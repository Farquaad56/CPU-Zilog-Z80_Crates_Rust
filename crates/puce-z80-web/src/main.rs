pub mod args;
pub mod http;
pub mod ws;

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use puce_z80_machines::session::Command;

/// The line printed once the server is listening.
pub fn banner(port: u16) -> String {
    format!("Zilog Z80 Emulator: http://127.0.0.1:{}/", port)
}

/// Watch stdin: a line holding `q` asks for a clean stop. End of input is not a request to stop
/// (the server may run detached, with no terminal).
fn watch_stdin(quit: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(l) if l.trim().eq_ignore_ascii_case("q") => {
                    quit.store(true, Ordering::SeqCst);
                    return;
                }
                Ok(_) => {}
                Err(_) => return,
            }
        }
    });
}

/// Serve until `quit` is set, then stop the server and the simulation thread.
fn run(a: &args::Args, quit: &AtomicBool) -> Result<(), String> {
    let ctx = Arc::new(http::Ctx::new(a.roms_dir.clone()));
    let mut server = http::start_with(a.port, Arc::clone(&ctx))?;
    println!("{}", banner(server.addr().port()));
    println!("(type q + Enter to stop)");
    while !quit.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
    }
    server.shutdown();
    ctx.session.send(Command::Quit);
    ctx.session.join();
    Ok(())
}

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let a = match args::parse_args(&raw_args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let quit = Arc::new(AtomicBool::new(false));
    watch_stdin(Arc::clone(&quit));
    if let Err(e) = run(&a, &quit) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn e14_08_startup_message() {
        assert_eq!(banner(8080), "Zilog Z80 Emulator: http://127.0.0.1:8080/");
        assert_eq!(banner(1), "Zilog Z80 Emulator: http://127.0.0.1:1/");
    }

    #[test]
    fn e14_08_quit_command() {
        let ctx = http::Ctx::new(PathBuf::from("no-such-roms-dir"));
        assert!(!ctx.session.finished());
        ctx.session.send(Command::Quit);
        ctx.session.join();
        assert!(ctx.session.finished());
    }

    #[test]
    fn e14_08_run_stops_when_asked() {
        let _g = crate::http::ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let a = args::Args {
            port: 0,
            roms_dir: PathBuf::from("no-such-roms-dir"),
        };
        let quit = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&quit);
        let t = std::thread::spawn(move || run(&a, &flag));
        std::thread::sleep(Duration::from_millis(250));
        quit.store(true, Ordering::SeqCst);
        assert!(t.join().unwrap().is_ok());
    }
}
