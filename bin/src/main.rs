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

fn setup_tracing() {
    use tracing_subscriber::{Registry, layer::SubscriberExt};
    use tracing_tree::HierarchicalLayer;

    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stderr)
        .with_indent_lines(true)
        .with_indent_amount(2)
        .with_verbose_exit(true)
        .with_verbose_entry(true)
        .with_targets(true);

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn setup_miette() {
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

    #[cfg(debug_assertions)]
    setup_tracing();

    setup_miette();

    match opts.command {
        Command::Build(opts) => cmd::build::handle(opts),
        Command::Run(opts) => cmd::run::handle(opts),
        Command::Dump(opts) => cmd::dump::handle(opts),
    }
}
