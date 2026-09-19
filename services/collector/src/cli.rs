use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "collector", version, about = "Web Analytics event collector")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Serve(ServeArgs),
    Migrate,
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeyCommands {
    Generate(KeyGenerateArgs),
}

#[derive(Debug, Args)]
pub struct KeyGenerateArgs {
    #[arg(long)]
    pub site: String,

    #[arg(long)]
    pub environment: String,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, env = "COLLECTOR_CONFIG")]
    pub config: PathBuf,

    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long, default_value_t = 4001)]
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{Cli, Commands};
    use clap::Parser;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn serve_uses_default_host_and_port() {
        let cli = Cli::try_parse_from(["collector", "serve", "--config", "config.toml"])
            .expect("CLI should parse");

        let Commands::Serve(args) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.config.to_string_lossy(), "config.toml");
        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, 4001);
    }

    #[test]
    fn serve_arguments_override_defaults() {
        let cli = Cli::try_parse_from([
            "collector",
            "serve",
            "--config",
            "config.toml",
            "--host",
            "127.0.0.1",
            "--port",
            "4100",
        ])
        .expect("CLI should parse");

        let Commands::Serve(args) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 4100);
    }

    #[test]
    fn config_uses_collector_config_environment_variable() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous = std::env::var_os("COLLECTOR_CONFIG");

        // Environment mutation is unsafe in Rust 2024 because it is process-global.
        unsafe { std::env::set_var("COLLECTOR_CONFIG", "from-env.toml") };
        let cli = Cli::try_parse_from(["collector", "serve"])
            .expect("CLI should read config from the environment");

        let Commands::Serve(args) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.config.to_string_lossy(), "from-env.toml");

        restore_config_env(previous);
    }

    #[test]
    fn config_cli_argument_overrides_environment_variable() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous = std::env::var_os("COLLECTOR_CONFIG");

        // Environment mutation is unsafe in Rust 2024 because it is process-global.
        unsafe { std::env::set_var("COLLECTOR_CONFIG", "from-env.toml") };
        let cli = Cli::try_parse_from(["collector", "serve", "--config", "from-cli.toml"])
            .expect("CLI argument should override the environment");

        let Commands::Serve(args) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.config.to_string_lossy(), "from-cli.toml");

        restore_config_env(previous);
    }

    #[test]
    fn key_generate_requires_site_and_environment() {
        assert!(Cli::try_parse_from(["collector", "key", "generate"]).is_err());
        assert!(
            Cli::try_parse_from(["collector", "key", "generate", "--site", "site_example"])
                .is_err()
        );
    }

    fn restore_config_env(previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => {
                // Environment mutation is unsafe in Rust 2024 because it is process-global.
                unsafe { std::env::set_var("COLLECTOR_CONFIG", value) };
            }
            None => {
                // Environment mutation is unsafe in Rust 2024 because it is process-global.
                unsafe { std::env::remove_var("COLLECTOR_CONFIG") };
            }
        }
    }
}
