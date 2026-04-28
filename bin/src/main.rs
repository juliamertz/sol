use clap::Parser;
use miette::Result;

use crate::cmd::build::BuildOpts;
use crate::cmd::dump::DumpOpts;

mod cmd;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Build(BuildOpts),
    Run(BuildOpts),
    Dump(DumpOpts),
}

fn init_miette() {
    miette::set_hook(Box::new(|_| {
        let theme = miette::GraphicalTheme {
            characters: miette::ThemeCharacters::unicode(),
            styles: miette::ThemeStyles::ansi(),
        };
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .context_lines(3)
                .graphical_theme(theme)
                .build(),
        )
    }))
    .unwrap();
}

fn main() -> Result<()> {
    let opts = Cli::parse();
    init_miette();

    match opts.command {
        Command::Build(opts) => cmd::build::handle(opts),
        Command::Run(opts) => cmd::run::handle(opts),
        Command::Dump(opts) => cmd::dump::handle(opts),
    }
}
