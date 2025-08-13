use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use tracing::{info, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use waagent_core::network::firewall::{create_firewall_manager, Action, Direction, FirewallRule, Protocol};

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum LoggingLevel {
    Debug,
    Info,
    Warn,
    Error,
}

// Implement Display so we can convert to string for tracing
impl fmt::Display for LoggingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoggingLevel::Debug => write!(f, "debug"),
            LoggingLevel::Info => write!(f, "info"),
            LoggingLevel::Warn => write!(f, "warn"),
            LoggingLevel::Error => write!(f, "error"),
        }
    }
}

/// Azure agent for configuring firewall rules and managing logging levels.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whether to configure the firewall rules
    #[arg(long, default_value_t = false)]
    configure_firewall: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, value_enum, default_value = "info")]
    log_level: LoggingLevel,

    /// Whether to log line numbers or not
    #[arg(long, default_value_t = false)]
    log_line_numbers: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize tracing
    init_tracing(&args.log_level, &args.log_line_numbers)?;

    // Output args if debug level logging
    debug!("Parsed arguments: {:?}", args);

    if args.configure_firewall {
        configure_firewall().await?;
    }

    Ok(())
}

fn init_tracing(log_level: &LoggingLevel, log_line_numbers: &bool) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level.to_string()));

    let mut format = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true);

    // Include file and line number if debug level
    if *log_line_numbers {
        format = format
            .with_file(true)
            .with_line_number(true);
    }

    tracing_subscriber::registry()
        .with(format)
        .with(env_filter)
        .init();

    debug!("Tracing initialized with level: {}", log_level);
    Ok(())
}

#[tracing::instrument]
async fn configure_firewall() -> Result<()> {
    info!("Configure firewall rules ...");

    let firewall_manager = create_firewall_manager();
    
    let rule = FirewallRule {
        name: "AllowAzureMetadata".into(),
        direction: Direction::Outbound,
        action: Action::Allow,
        protocol: Protocol::Tcp,
        destination: "168.63.129.16/32".into(),
        port: None,
        uid_owner: Some("999".into()), // UID for unix
        program_path: None,
    };

    debug!("Adding firewall rule: {:?}", rule);
    let result = firewall_manager.add_rule(&rule);
    
    if result.is_err() {
        let error = result.unwrap_err();
        error!("Failed to add firewall rule: {:?}", error);
        return Err(anyhow::anyhow!("Failed to add firewall rule: {}", error));
    }

    info!("Firewall rule added successfully");

    Ok(())
}