pub mod args;
pub mod http;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    match args::parse_args(&raw_args) {
        Ok(a) => {
            println!("z80-web: port {}, roms {}", a.port, a.roms_dir.display());
            match http::start(a.port) {
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
