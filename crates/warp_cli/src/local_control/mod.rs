//! Command-line interface for controlling a running local Warp app.
mod commands;
mod completions;
mod output;
mod selectors;

use std::process::ExitCode;

use crate::agent::OutputFormat;
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::aot::Shell;

use commands::{
    run_action_command, run_app_command, run_capability_command, run_instance_command,
    run_pane_command, run_session_command, run_tab_command, run_window_command,
};
use completions::generate_completions_to_stdout;
use output::write_control_error;

/// Parsed top-level arguments for `warpctrl`.
#[derive(Debug, Parser)]
#[command(
    name = "warpctrl",
    display_name = "warpctrl",
    about = "Control a running local Warp app instance"
)]
pub struct ControlArgs {
    /// Set the output format.
    #[arg(
        long = "output-format",
        global = true,
        value_enum,
        default_value_t = OutputFormat::Pretty,
        env = "WARP_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: ControlCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CapabilityCommand {
    /// List implemented local-control capabilities.
    List(TargetArgs),

    /// Inspect one local-control capability.
    Inspect(ActionGetArgs),
}

impl ControlArgs {
    pub fn from_env() -> Self {
        let matches = Self::clap_command().get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
    }

    pub fn clap_command() -> clap::Command {
        let bin_name = crate::binary_name().unwrap_or_else(|| "warpctrl".to_owned());
        <Self as CommandFactory>::command()
            .version(crate::version_string())
            .bin_name(bin_name.clone())
            .after_help(color_print::cformat!(
                r#"<bold><underline>Examples:</underline></bold>

  <dim>$</dim> <bold>{bin_name} instance list</bold>

  <dim>$</dim> <bold>{bin_name} tab create</bold>

<bold><underline>Learn more:</underline></bold>
* Use <bold>{bin_name} help</bold> to learn more about each command
"#
            ))
    }
}

/// Top-level `warpctrl` command groups.
#[derive(Debug, Clone, Subcommand)]
pub enum ControlCommand {
    /// Inspect local Warp app instances.
    #[command(subcommand)]
    Instance(InstanceCommand),
    /// Inspect a selected local Warp app.
    #[command(subcommand)]
    App(AppCommand),
    /// Inspect the local-control action catalog.
    #[command(subcommand)]
    Action(ActionCommand),

    /// Inspect local-control capabilities.
    #[command(subcommand)]
    Capability(CapabilityCommand),

    /// Inspect local Warp windows.
    #[command(subcommand)]
    Window(WindowCommand),

    /// Control local Warp tabs.
    #[command(subcommand)]
    Tab(TabCommand),
    /// Inspect local Warp panes.
    #[command(subcommand)]
    Pane(PaneCommand),

    /// Inspect local Warp sessions.
    #[command(subcommand)]
    Session(SessionCommand),

    /// Generate shell completions for your shell to stdout.
    ///
    /// For bash, add the following to ~/.bashrc:
    ///     source <(path/to/warpctrl completions bash)
    ///
    /// For zsh, add the following to ~/.zshrc:
    ///     source <(path/to/warpctrl completions zsh)
    ///
    /// For fish, add the following to ~/.config/fish/config.fish:
    ///     path/to/warpctrl completions fish | source
    ///
    /// For Powershell, add the following to $PROFILE:
    ///     path\to\warpctrl completions powershell | Out-String | Invoke-Expression
    ///
    /// If no shell is provided, this defaults to the shell that Warp was run from.
    #[command(verbatim_doc_comment)]
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

/// Commands that inspect locally discoverable Warp instances.
#[derive(Debug, Clone, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum InstanceCommand {
    /// List locally discoverable Warp instances.
    List,

    /// Inspect the selected local Warp app instance.
    Inspect(TargetArgs),
}

/// Commands that inspect the selected Warp app instance.
#[derive(Debug, Clone, Subcommand)]
pub enum AppCommand {
    /// Check that the selected local Warp app responds.
    Ping(TargetArgs),

    /// Print protocol and app version metadata for the selected local Warp app.
    Version(TargetArgs),

    /// Print the active window/tab/pane/session chain.
    Active(TargetArgs),

    /// Print app and protocol metadata.
    Inspect(TargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ActionCommand {
    /// List allowlisted local-control actions.
    List(TargetArgs),

    /// Inspect one allowlisted local-control action.
    Get(ActionGetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum WindowCommand {
    /// List windows in the selected local Warp app.
    List(TargetArgs),

    /// Inspect one window in the selected local Warp app.
    Inspect(TargetArgs),
}

/// Commands that control tabs in the selected Warp app instance.
#[derive(Debug, Clone, Subcommand)]
pub enum TabCommand {
    /// List tabs in the selected local Warp app.
    List(TargetArgs),
    /// Inspect one tab in the selected local Warp app.
    Inspect(TargetArgs),
    /// Create a new terminal tab in the active window.
    Create(TargetArgs),
}

/// Commands that inspect local Warp panes.
#[derive(Debug, Clone, Subcommand)]
pub enum PaneCommand {
    /// List panes in the selected local Warp app.
    List(TargetArgs),
    /// Inspect one pane in the selected local Warp app.
    Inspect(TargetArgs),
}
/// Commands that inspect local Warp sessions.

#[derive(Debug, Clone, Subcommand)]
pub enum SessionCommand {
    /// List sessions in the selected local Warp app.
    List(TargetArgs),
    /// Inspect one session in the selected local Warp app.
    Inspect(TargetArgs),
}
/// Common flags for selecting which running Warp instance receives a command.
#[derive(Debug, Clone, Args, Default)]
pub struct TargetArgs {
    /// Target a specific local Warp instance id from `warp instance list`.
    #[arg(long = "instance")]
    pub instance: Option<String>,

    /// Target a specific local Warp process id.
    #[arg(long = "pid", conflicts_with = "instance")]
    pub pid: Option<u32>,

    /// Target a window selector: active, id:<id>, index:<n>, or title:<title>.
    #[arg(long = "window", conflicts_with_all = ["window_id", "window_index", "window_title"])]
    pub window: Option<String>,

    /// Target an opaque window id from `warpctrl window list`.
    #[arg(long = "window-id", conflicts_with_all = ["window", "window_index", "window_title"])]
    pub window_id: Option<String>,

    /// Target a window by its list index.
    #[arg(long = "window-index", conflicts_with_all = ["window", "window_id", "window_title"])]
    pub window_index: Option<u32>,

    /// Target a window by exact title.
    #[arg(long = "window-title", conflicts_with_all = ["window", "window_id", "window_index"])]
    pub window_title: Option<String>,

    /// Target a tab selector: active, id:<id>, index:<n>, or title:<title>.
    #[arg(long = "tab", conflicts_with_all = ["tab_id", "tab_index", "tab_title"])]
    pub tab: Option<String>,

    /// Target an opaque tab id from `warpctrl tab list`.
    #[arg(long = "tab-id", conflicts_with_all = ["tab", "tab_index", "tab_title"])]
    pub tab_id: Option<String>,

    /// Target a tab by its window-scoped index.
    #[arg(long = "tab-index", conflicts_with_all = ["tab", "tab_id", "tab_title"])]
    pub tab_index: Option<u32>,

    /// Target a tab by exact title.
    #[arg(long = "tab-title", conflicts_with_all = ["tab", "tab_id", "tab_index"])]
    pub tab_title: Option<String>,

    /// Target a pane selector: active, id:<id>, or index:<n>.
    #[arg(long = "pane", conflicts_with_all = ["pane_id", "pane_index"])]
    pub pane: Option<String>,

    /// Target an opaque pane id from `warpctrl pane list`.
    #[arg(long = "pane-id", conflicts_with_all = ["pane", "pane_index"])]
    pub pane_id: Option<String>,

    /// Target a pane by its tab-scoped index.
    #[arg(long = "pane-index", conflicts_with_all = ["pane", "pane_id"])]
    pub pane_index: Option<u32>,

    /// Target a session selector: active, id:<id>, or index:<n>.
    #[arg(long = "session", conflicts_with_all = ["session_id", "session_index"])]
    pub session: Option<String>,

    /// Target an opaque session id from `warpctrl session list`.
    #[arg(long = "session-id", conflicts_with_all = ["session", "session_index"])]
    pub session_id: Option<String>,

    /// Target a session by its pane-scoped index.
    #[arg(long = "session-index", conflicts_with_all = ["session", "session_id"])]
    pub session_index: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct ActionGetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Action name, such as tab.create or window.list.
    pub action: String,
}

pub fn run(args: ControlArgs) -> ExitCode {
    let output_format = args.output_format;
    match run_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Err(write_error) = write_control_error(&error, output_format) {
                eprintln!(
                    "error: failed to render local-control error: {}",
                    write_error.message
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: ControlArgs) -> Result<(), local_control::protocol::ControlError> {
    let output_format = args.output_format;
    match args.command {
        ControlCommand::Instance(command) => run_instance_command(command, output_format),
        ControlCommand::App(command) => run_app_command(command, output_format),
        ControlCommand::Action(command) => run_action_command(command, output_format),
        ControlCommand::Capability(command) => run_capability_command(command, output_format),
        ControlCommand::Window(command) => run_window_command(command, output_format),
        ControlCommand::Tab(command) => run_tab_command(command, output_format),
        ControlCommand::Pane(command) => run_pane_command(command, output_format),
        ControlCommand::Session(command) => run_session_command(command, output_format),
        ControlCommand::Completions { shell } => generate_completions_to_stdout(shell),
    }
}

#[cfg(test)]
pub(crate) use completions::generate_completion_string;
#[cfg(test)]
pub(crate) use output::ErrorSummary;

#[cfg(test)]
#[path = "../local_control_tests.rs"]
mod tests;
