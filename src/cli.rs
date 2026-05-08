use clap::Parser;

#[derive(Parser)]
#[command(
    name = "cdx",
    about = "Interactive directory navigator",
    disable_help_flag = true
)]
pub struct Cli {
    #[arg(short = 'g', long)]
    pub grep: bool,

    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help)]
    pub help: (),

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub query: Vec<String>,
}
