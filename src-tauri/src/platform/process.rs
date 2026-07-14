//! Safe construction for non-interactive child processes.
//!
//! Mr Manager captures child output itself. On Windows, launching a console
//! executable directly from the GUI process would otherwise create a visible
//! console window for every probe or refresh.

use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a command intended for captured, non-interactive execution.
///
/// Callers still own the exact argument vector, stdio policy, timeout, and
/// cancellation. This helper only applies the platform windowing policy.
pub fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    configure_hidden(&mut command);
    command
}

pub fn configure_hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    const CHILD_CONSOLE_TEST_ENV: &str = "MR_MANAGER_CHILD_CONSOLE_TEST";

    #[cfg(windows)]
    #[test]
    fn windows_child_policy_uses_create_no_window() {
        assert_eq!(super::CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[cfg(windows)]
    #[test]
    fn windows_child_policy_spawns_without_a_console() {
        let current_executable =
            std::env::current_exe().expect("the Rust test executable path should be available");
        let output = super::hidden_command(current_executable)
            .args([
                "--exact",
                "platform::process::tests::child_process_has_no_console_window",
                "--nocapture",
            ])
            .env(CHILD_CONSOLE_TEST_ENV, "1")
            .output()
            .expect("the child console test should launch");

        assert!(
            output.status.success(),
            "hidden child unexpectedly had a console: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(windows)]
    #[test]
    fn child_process_has_no_console_window() {
        if std::env::var_os(CHILD_CONSOLE_TEST_ENV).is_none() {
            return;
        }

        unsafe extern "system" {
            fn GetConsoleWindow() -> *mut std::ffi::c_void;
        }

        // SAFETY: GetConsoleWindow has no arguments and only returns the calling
        // process's console-window handle, if one exists.
        let console_window = unsafe { GetConsoleWindow() };
        assert!(console_window.is_null());
    }
}
