use crate::commands::format::apply_format_settings_from_cli;
use crate::{execute_mode, CliSession, Execution, Termination, TraversalMode};
use rome_diagnostics::MAXIMUM_DISPLAYABLE_DIAGNOSTICS;
use rome_service::load_config;
use rome_service::settings::WorkspaceSettings;
use rome_service::workspace::{FixFileMode, UpdateSettingsParams};

/// Handler for the "check" command of the Rome CLI
pub(crate) fn check(mut session: CliSession) -> Result<(), Termination> {
    let configuration = load_config(&session.app.fs, None)?;
    let mut workspace_settings = WorkspaceSettings::default();

    let max_diagnostics: Option<u16> = session
        .args
        .opt_value_from_str("--max-diagnostics")
        .map_err(|source| Termination::ParseError {
            argument: "--max-diagnostics",
            source,
        })?;
    let max_diagnostics = if let Some(max_diagnostics) = max_diagnostics {
        if max_diagnostics > MAXIMUM_DISPLAYABLE_DIAGNOSTICS {
            return Err(Termination::OverflowNumberArgument(
                "--max-diagnostics",
                MAXIMUM_DISPLAYABLE_DIAGNOSTICS,
            ));
        }

        max_diagnostics
    } else {
        // default value
        20
    };

    if let Some(configuration) = configuration {
        session
            .app
            .workspace
            .update_settings(UpdateSettingsParams { configuration })?;
    }
    apply_format_settings_from_cli(&mut session, &mut workspace_settings)?;

    let apply = session.args.contains("--apply");
    let apply_suggested = session.args.contains("--apply-suggested");

    let fix_file_mode = if apply && apply_suggested {
        return Err(Termination::IncompatibleArguments(
            "--apply",
            "--apply-suggested",
        ));
    } else if !apply && !apply_suggested {
        None
    } else if apply && !apply_suggested {
        Some(FixFileMode::SafeFixes)
    } else {
        Some(FixFileMode::SafeAndSuggestedFixes)
    };

    execute_mode(
        Execution::new(TraversalMode::Check {
            max_diagnostics,
            fix_file_mode,
        }),
        session,
    )
}
