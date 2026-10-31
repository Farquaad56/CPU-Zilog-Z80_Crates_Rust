pub mod args;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    match args::parse_args(&raw_args) {
        Ok(a) => {
            println!("z80-web: port {}, roms {}", a.port, a.roms_dir.display());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
