use std::{path::PathBuf, sync::LazyLock};
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug, Clone)]
pub enum AdminCommand {
    /// List all admin users
    List,

    /// Change an admin user's password
    ChangePassword {
        #[arg(long)]
        name: String,
        #[arg(long)]
        new_password: String,
    },

    /// Create a new admin user
    New {
        #[arg(long)]
        name: String,
        #[arg(long)]
        password: String,
    },

    /// Delete an admin user
    Delete {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BoardCategoryCommand {
    /// Create a new board category
    New {
        #[arg(long)]
        name: String,
    },

    /// List all board categories
    List,

    /// Delete a board category
    Delete {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BoardCommand {
    /// Create a new board
    New {
        #[arg(long)]
        slug: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        category: Option<String>,
    },

    /// List all boards
    List,

    /// Delete a board
    Delete {
        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CliAction {
    /// Start the web server
    Serve,
    #[command(subcommand)]
    Admin(AdminCommand),
    #[command(subcommand)]
    Board(BoardCommand),
    #[command(subcommand)]
    Category(BoardCategoryCommand),
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