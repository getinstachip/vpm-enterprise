use clap::Parser;

#[derive(Debug, Parser)]
pub enum Cmd {
    Install(Install),
    Uninstall(Uninstall),
    Chat(Chat),
    Docs(Docs),
    Dotf(Dotf),
    List(List),
    Tune(Tune),
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    disable_version_flag = true,
    propagate_version = true,
    version
)]
pub struct Install {
    #[arg(help = "Name of package to install")]
    pub package_name: Option<String>,
    #[arg(help = "URL of repository to install from")]
    pub url: Option<String>,
    #[arg(help="Version of package to install")]
    pub version: Option<String>,
    #[arg(long, help = "Enable flex installation")]
    pub flex: bool,
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version
)]
pub struct Uninstall {
    #[arg(help = "Name of package to Uninstall")]
    pub package_name: String,
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version
)]
pub struct Docs{
    #[arg(help = "Name of module to generate documentation for")]
    pub module: Option<String>,
    #[arg(help = "Url of repository to generate documentation for")]
    pub url: Option<String>,
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version
)]
pub struct Dotf {
    #[arg(help = "Path to top module to generate filelist for")]
    pub path_to_top_module: String,
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version,
)]
pub struct Chat {
    #[arg(help="Chat with file")]
    pub module_name: String,
    #[arg(help="Message to send to module")]
    pub message: String,
}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version,
)]
pub struct List {}

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    disable_help_subcommand = true,
    propagate_version = true,
    version,
)]
pub struct Tune {
    #[arg(help="Source module to tune")]
    pub source_module: String,
    #[arg(help="Target module to tune")]
    pub target_module: String,
}
