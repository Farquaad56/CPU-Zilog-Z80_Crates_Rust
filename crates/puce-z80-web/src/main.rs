pub mod args;
pub mod http;
pub mod ws;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    match args::parse_args(&raw_args) {
        Ok(a) => {
            println!("z80-web: port {}, roms {}", a.port, a.roms_dir.display());
            let ctx = std::sync::Arc::new(http::Ctx::new(a.roms_dir.clone()));
            match http::start_with(a.port, ctx) {
                Ok(mut server) => {
                    println!("z80-web: listening on http://{}/", server.addr());
                    server.wait();
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
