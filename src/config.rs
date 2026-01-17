use std::{path::PathBuf, sync::LazyLock};
use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Subcommand)]
pub enum CliAction {
    /// Start the web server
    Serve,
    /// Create a new admin user
    NewAdmin {
        #[arg(long)]
        name: String,

        #[arg(long)]
        password: String,
    },
    /// Delete an admin user
    DeleteAdmin {
        #[arg(long)]
        name: String,
    },
    /// Clean up the database
    Clean,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "tewi", about = "A web application")]
pub struct Cli {
    #[command(subcommand)]
    pub action: Option<CliAction>,

    #[arg(short, long, default_value = "3000", global = true)]
    pub port: u16,

    #[arg(long, default_value = "attachments", global = true)]
    pub attachments_folder: PathBuf,
    #[arg(long, default_value = "thumbnails", global = true)]
    pub thumbnails_folder: PathBuf,
}

pub static CONFIG: LazyLock<Cli> = LazyLock::new(|| {
    Cli::parse()
});