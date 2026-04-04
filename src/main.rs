use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terminalsort", about = "Tile terminal windows with font scaling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tile selected terminal windows on a monitor
    Tile {
        /// Number of windows to pick, or '*' for all
        #[arg(long)]
        pick: String,

        /// Layout: h2, v2, h3, v3, grid
        #[arg(long)]
        layout: String,

        /// Monitor index (0-based)
        #[arg(long)]
        monitor: usize,
    },
    /// List terminal windows and monitors
    List,
    /// Restore original font sizes
    Reset,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tile { pick, layout, monitor } => {
            println!("tile: pick={pick}, layout={layout}, monitor={monitor}");
        }
        Commands::List => {
            println!("list");
        }
        Commands::Reset => {
            println!("reset");
        }
    }

    Ok(())
}
