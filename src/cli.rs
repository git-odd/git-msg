use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "git-msg",
    bin_name = "git-msg",
    author,
    version,
    about = "AI-powered Git commit message generator",
    long_about = "A smart, lightweight, local-first Git commit message generator compatible with OpenAI-format endpoints."
)]
pub struct Cli {
    #[command(flatten)]
    pub run_args: RunArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct RunArgs {
    /// Skip interactive confirmation and auto-commit
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Print generated message to stdout without committing or staging
    #[arg(short = 'd', long = "dry-run")]
    pub dry_run: bool,

    /// Template name to use (conventional, simple, gitmoji)
    #[arg(short = 't', long = "template")]
    pub template: Option<String>,

    /// Override model name (e.g. qwen3.5-2b)
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,

    /// Override endpoint URL (e.g. http://127.0.0.1:1234)
    #[arg(short = 'e', long = "endpoint")]
    pub endpoint: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Open or inspect the global configuration file
    Config(ConfigArgs),

    /// Initialize a .gitmsg.toml configuration file in the current Git repository
    Init,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Print the absolute path to the configuration file without opening the editor
    #[arg(long = "show-path")]
    pub show_path: bool,
}
