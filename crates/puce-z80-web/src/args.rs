use std::path::PathBuf;

pub struct Args {
    pub port: u16,
    pub roms_dir: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            port: 8080,
            roms_dir: PathBuf::from("tests/roms"),
        }
    }
}

pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut result = Args::default();

    let mut i = 1; // skip program name
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 >= args.len() {
                    return Err("--port requires a value".to_string());
                }
                let port_str = &args[i + 1];
                result.port = port_str
                    .parse::<u16>()
                    .map_err(|_| format!("invalid port: {}", port_str))?;
                i += 2;
            }
            "--roms" => {
                if i + 1 >= args.len() {
                    return Err("--roms requires a value".to_string());
                }
                result.roms_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--help" => {
                println!("Usage: z80-web [OPTIONS]");
                println!("Options:");
                println!("  --port N      Port to listen on (default: 8080)");
                println!("  --roms DIR    ROM directory (default: tests/roms)");
                println!("  --help        Show this help message");
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown flag: {}", other));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e01_03_defaults() {
        let result = parse_args(&[]).unwrap();
        assert_eq!(result.port, 8080);
        assert_eq!(result.roms_dir.to_string_lossy(), "tests/roms");
    }

    #[test]
    fn e01_03_port_ok() {
        let result =
            parse_args(&["prog".to_string(), "--port".to_string(), "9000".to_string()]).unwrap();
        assert_eq!(result.port, 9000);
    }

    #[test]
    fn e01_03_port_bad() {
        let result = parse_args(&["prog".to_string(), "--port".to_string(), "abc".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn e01_03_unknown_flag() {
        let result = parse_args(&["prog".to_string(), "--foo".to_string()]);
        assert!(result.is_err());
    }
}
