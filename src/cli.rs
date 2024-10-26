pub fn get_clap_command() -> clap::Command {
    clap::Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            clap::Arg::new("fullscreen")
                .short('f')
                .long("fullscreen")
                .help("Start simp in fullscreen")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("generate-man")
                .long("generate-man")
                .help("Generates manual page for simp")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("zen-mode")
                .long("zen-mode")
                .short('z')
                .help("Remove all UI")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("no-cache")
                .long("no-cache")
                .help("Do not cache images in memory")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            clap::Arg::new("class")
                .long("class")
                .default_value("simp")
                .help("Defines window class/app_id on X11/Wayland"),
        )
        .arg(clap::Arg::new("file").help("Load this file").index(1))
}
