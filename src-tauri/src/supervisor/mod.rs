use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    ManagedCommand, ManagedCommandLogEntry, ManagedCommandState, ManagedCommandStream, Project,
    ProjectScript,
};
use crate::projects::now_ms;
use crate::security::redaction;

const MAX_LOG_ENTRIES: usize = 2_000;
const MAX_LOG_LINE_CHARS: usize = 4_000;
const GRACEFUL_STOP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("the managed process supervisor is unavailable")]
    LockUnavailable,
    #[error("the requested project command does not exist")]
    ScriptNotFound,
    #[error("the command working directory is outside the registered project root")]
    UnsafeWorkingDirectory,
    #[error("the command executable is empty")]
    EmptyExecutable,
    #[error("the managed process could not be started")]
    Spawn(#[source] io::Error),
    #[error("the Windows Job Object could not be created or assigned: {0}")]
    JobObject(String),
    #[error("the managed process run does not exist")]
    RunNotFound,
    #[error("the stop request could not be delivered")]
    Stop(#[source] io::Error),
}

#[derive(Clone, Default)]
pub struct ProcessSupervisor {
    runs: Arc<Mutex<HashMap<String, Arc<Mutex<RunState>>>>>,
}

impl ProcessSupervisor {
    pub fn run_project_command(
        &self,
        project: &Project,
        script_id: &str,
    ) -> Result<ManagedCommand, SupervisorError> {
        let script = project
            .scripts
            .iter()
            .find(|script| script.id == script_id)
            .ok_or(SupervisorError::ScriptNotFound)?
            .clone();
        validate_script(project, &script)?;

        let run_id = Uuid::new_v4().to_string();
        let mut command = crate::platform::process::hidden_command(&script.executable);
        command
            .args(&script.arguments)
            .current_dir(&script.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(SupervisorError::Spawn)?;
        let pid = child.id();
        let job = JobObject::create_and_assign(&child)?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let started_at_ms = now_ms();
        let summary = ManagedCommand {
            run_id: run_id.clone(),
            project_id: project.id.clone(),
            script_id: script.id.clone(),
            label: script.label.clone(),
            executable: script.executable.clone(),
            arguments: script.arguments.clone(),
            working_directory: script.working_directory.clone(),
            pid: Some(pid),
            started_at_ms,
            ended_at_ms: None,
            state: ManagedCommandState::Running,
            exit_code: None,
            stop_requested: false,
            log_count: 0,
        };
        let state = Arc::new(Mutex::new(RunState {
            summary,
            child: Some(child),
            job: Some(job),
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            next_sequence: 0,
        }));
        {
            let mut runs = self
                .runs
                .lock()
                .map_err(|_| SupervisorError::LockUnavailable)?;
            runs.insert(run_id.clone(), Arc::clone(&state));
        }

        append_log(
            &state,
            ManagedCommandStream::System,
            format!(
                "Started PID {pid}: {} {}",
                script.executable,
                script.arguments.join(" ")
            ),
        );
        spawn_log_reader(Arc::clone(&state), ManagedCommandStream::Stdout, stdout);
        spawn_log_reader(state, ManagedCommandStream::Stderr, stderr);

        self.get_run(&run_id)
    }

    pub fn list_runs(&self) -> Result<Vec<ManagedCommand>, SupervisorError> {
        let states = self.run_states()?;
        let mut runs = Vec::new();
        for state in states {
            let mut state = state.lock().map_err(|_| SupervisorError::LockUnavailable)?;
            poll_run(&mut state);
            runs.push(state.summary.clone());
        }
        runs.sort_by(|left, right| right.started_at_ms.cmp(&left.started_at_ms));
        Ok(runs)
    }

    pub fn get_run(&self, run_id: &str) -> Result<ManagedCommand, SupervisorError> {
        let state = self.run_state(run_id)?;
        let mut state = state.lock().map_err(|_| SupervisorError::LockUnavailable)?;
        poll_run(&mut state);
        Ok(state.summary.clone())
    }

    pub fn logs(&self, run_id: &str) -> Result<Vec<ManagedCommandLogEntry>, SupervisorError> {
        let state = self.run_state(run_id)?;
        let mut state = state.lock().map_err(|_| SupervisorError::LockUnavailable)?;
        poll_run(&mut state);
        Ok(state.logs.iter().cloned().collect())
    }

    pub fn stop_run(&self, run_id: &str, force: bool) -> Result<ManagedCommand, SupervisorError> {
        let state = self.run_state(run_id)?;
        let mut state = state.lock().map_err(|_| SupervisorError::LockUnavailable)?;
        poll_run(&mut state);
        if matches!(
            state.summary.state,
            ManagedCommandState::Exited | ManagedCommandState::Failed
        ) {
            return Ok(state.summary.clone());
        }

        state.summary.stop_requested = true;
        state.summary.state = ManagedCommandState::Stopping;
        if force {
            if let Some(job) = &state.job {
                job.terminate(1)?;
            }
            if let Some(child) = &mut state.child {
                let _ = child.kill();
            }
            push_log(
                &mut state,
                ManagedCommandStream::System,
                "Force-stop requested for the managed process tree.",
            );
        } else if let Some(pid) = state.summary.pid {
            request_graceful_stop(pid)?;
            push_log(
                &mut state,
                ManagedCommandStream::System,
                "Graceful stop requested for the managed process tree.",
            );
        }
        poll_run(&mut state);
        Ok(state.summary.clone())
    }

    fn run_state(&self, run_id: &str) -> Result<Arc<Mutex<RunState>>, SupervisorError> {
        self.runs
            .lock()
            .map_err(|_| SupervisorError::LockUnavailable)?
            .get(run_id)
            .cloned()
            .ok_or(SupervisorError::RunNotFound)
    }

    fn run_states(&self) -> Result<Vec<Arc<Mutex<RunState>>>, SupervisorError> {
        Ok(self
            .runs
            .lock()
            .map_err(|_| SupervisorError::LockUnavailable)?
            .values()
            .cloned()
            .collect())
    }
}

struct RunState {
    summary: ManagedCommand,
    child: Option<Child>,
    job: Option<JobObject>,
    logs: VecDeque<ManagedCommandLogEntry>,
    next_sequence: u64,
}

fn validate_script(project: &Project, script: &ProjectScript) -> Result<(), SupervisorError> {
    if script.executable.trim().is_empty() {
        return Err(SupervisorError::EmptyExecutable);
    }

    let working_directory = canonicalize_existing_directory(&script.working_directory)?;
    let project_root = canonicalize_existing_directory(&project.canonical_root_path)?;
    if !working_directory.starts_with(project_root) {
        return Err(SupervisorError::UnsafeWorkingDirectory);
    }

    Ok(())
}

fn canonicalize_existing_directory(path: &str) -> Result<PathBuf, SupervisorError> {
    let path = Path::new(path);
    let canonical = path
        .canonicalize()
        .map_err(|error| SupervisorError::Spawn(io::Error::new(error.kind(), error)))?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(SupervisorError::UnsafeWorkingDirectory)
    }
}

fn spawn_log_reader(
    state: Arc<Mutex<RunState>>,
    stream: ManagedCommandStream,
    pipe: Option<impl io::Read + Send + 'static>,
) {
    if let Some(pipe) = pipe {
        thread::spawn(move || {
            let reader = BufReader::new(pipe);
            for line in reader.lines() {
                match line {
                    Ok(line) => append_log(&state, stream, line),
                    Err(error) => {
                        append_log(
                            &state,
                            ManagedCommandStream::System,
                            format!("Log stream ended with an error: {error}"),
                        );
                        break;
                    }
                }
            }
        });
    }
}

fn append_log(state: &Arc<Mutex<RunState>>, stream: ManagedCommandStream, line: impl AsRef<str>) {
    if let Ok(mut state) = state.lock() {
        push_log(&mut state, stream, line);
    }
}

fn push_log(state: &mut RunState, stream: ManagedCommandStream, line: impl AsRef<str>) {
    let redacted = redaction::redact(line.as_ref());
    let line = redacted
        .chars()
        .take(MAX_LOG_LINE_CHARS)
        .collect::<String>();
    let entry = ManagedCommandLogEntry {
        sequence: state.next_sequence,
        timestamp_ms: now_ms(),
        stream,
        line,
    };
    state.next_sequence = state.next_sequence.saturating_add(1);
    state.logs.push_back(entry);
    while state.logs.len() > MAX_LOG_ENTRIES {
        state.logs.pop_front();
    }
    state.summary.log_count = state.next_sequence;
}

fn poll_run(state: &mut RunState) {
    let Some(child) = &mut state.child else {
        return;
    };
    match child.try_wait() {
        Ok(Some(status)) => {
            state.summary.ended_at_ms = Some(now_ms());
            state.summary.exit_code = status.code();
            state.summary.state = if status.success() {
                ManagedCommandState::Exited
            } else {
                ManagedCommandState::Failed
            };
            state.child = None;
            state.job = None;
            push_log(
                state,
                ManagedCommandStream::System,
                format!("Process exited with status {status}."),
            );
        }
        Ok(None) => {
            if !matches!(state.summary.state, ManagedCommandState::Stopping) {
                state.summary.state = ManagedCommandState::Running;
            }
        }
        Err(error) => {
            state.summary.ended_at_ms = Some(now_ms());
            state.summary.state = ManagedCommandState::Failed;
            state.child = None;
            state.job = None;
            push_log(
                state,
                ManagedCommandStream::System,
                format!("Process status check failed: {error}"),
            );
        }
    }
}

#[cfg(windows)]
fn request_graceful_stop(pid: u32) -> Result<(), SupervisorError> {
    let mut child = crate::platform::process::hidden_command("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(SupervisorError::Stop)?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if started.elapsed() >= GRACEFUL_STOP_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => return Err(SupervisorError::Stop(error)),
        }
    }
}

#[cfg(not(windows))]
fn request_graceful_stop(_pid: u32) -> Result<(), SupervisorError> {
    Ok(())
}

#[cfg(windows)]
struct JobObject {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for JobObject {}

#[cfg(windows)]
impl JobObject {
    fn create_and_assign(child: &Child) -> Result<Self, SupervisorError> {
        use std::ffi::c_void;
        use std::mem::size_of;

        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };
        use windows::core::PCWSTR;

        // SAFETY: No security attributes or name are supplied. Windows returns
        // a handle owned by this wrapper, or an error without transferring one.
        let handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| SupervisorError::JobObject(error.to_string()))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `limits` is the documented structure for
        // JobObjectExtendedLimitInformation and remains alive for the call.
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if let Err(error) = configured {
            // SAFETY: The job handle was returned by CreateJobObjectW and is
            // not used after this close path.
            let _ = unsafe { CloseHandle(handle) };
            return Err(SupervisorError::JobObject(error.to_string()));
        }

        let process_handle = HANDLE(child.as_raw_handle());
        // SAFETY: The child process handle belongs to `child` and is valid for
        // the duration of this call. The job handle remains owned by `JobObject`.
        if let Err(error) = unsafe { AssignProcessToJobObject(handle, process_handle) } {
            // SAFETY: The job handle was returned by CreateJobObjectW and is
            // not used after this close path.
            let _ = unsafe { CloseHandle(handle) };
            return Err(SupervisorError::JobObject(error.to_string()));
        }

        Ok(Self { handle })
    }

    fn terminate(&self, exit_code: u32) -> Result<(), SupervisorError> {
        use windows::Win32::System::JobObjects::TerminateJobObject;

        // SAFETY: The handle is a live Job Object owned by this wrapper.
        unsafe { TerminateJobObject(self.handle, exit_code) }
            .map_err(|error| SupervisorError::JobObject(error.to_string()))
    }
}

#[cfg(windows)]
impl Drop for JobObject {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;

        // SAFETY: The handle is owned by this wrapper and is closed exactly once.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

#[cfg(not(windows))]
struct JobObject;

#[cfg(not(windows))]
impl JobObject {
    fn create_and_assign(_child: &Child) -> Result<Self, SupervisorError> {
        Ok(Self)
    }

    fn terminate(&self, _exit_code: u32) -> Result<(), SupervisorError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_redaction_bounds_secret_output() {
        let summary = ManagedCommand {
            run_id: "run".to_owned(),
            project_id: "project".to_owned(),
            script_id: "script".to_owned(),
            label: "script".to_owned(),
            executable: "tool".to_owned(),
            arguments: Vec::new(),
            working_directory: ".".to_owned(),
            pid: None,
            started_at_ms: 1,
            ended_at_ms: None,
            state: ManagedCommandState::Running,
            exit_code: None,
            stop_requested: false,
            log_count: 0,
        };
        let mut state = RunState {
            summary,
            child: None,
            job: None,
            logs: VecDeque::new(),
            next_sequence: 0,
        };

        push_log(
            &mut state,
            ManagedCommandStream::Stdout,
            "Authorization: Bearer secret-token",
        );

        assert_eq!(state.logs.len(), 1);
        assert!(!state.logs[0].line.contains("secret-token"));
        assert!(state.logs[0].line.contains("[REDACTED]"));
    }
}
