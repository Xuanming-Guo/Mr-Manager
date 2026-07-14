use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{
    AppError, BackgroundTask, BackgroundTaskDetail, BackgroundTaskKind, BackgroundTaskOutput,
    BackgroundTaskState,
};

const MAX_RETAINED_TASKS: usize = 64;

pub type TaskRegistry = Arc<Mutex<TaskManager>>;

#[derive(Debug, Default)]
pub struct TaskManager {
    tasks: HashMap<String, BackgroundTaskDetail>,
    order: VecDeque<String>,
}

impl TaskManager {
    pub fn start(
        &mut self,
        id: String,
        kind: BackgroundTaskKind,
        label: impl Into<String>,
        route: impl Into<String>,
        cancellable: bool,
    ) -> BackgroundTask {
        self.trim_completed();
        let task = BackgroundTask {
            id: id.clone(),
            kind,
            label: label.into(),
            route: route.into(),
            state: BackgroundTaskState::Running,
            started_at_ms: now_ms(),
            completed_at_ms: None,
            cancellable,
            progress_percent: None,
            summary: None,
            error: None,
        };
        self.order.retain(|existing| existing != &id);
        self.order.push_front(id.clone());
        self.tasks.insert(
            id,
            BackgroundTaskDetail {
                task: task.clone(),
                output: None,
            },
        );
        task
    }

    pub fn list(&self) -> Vec<BackgroundTask> {
        self.order
            .iter()
            .filter_map(|id| self.tasks.get(id).map(|detail| detail.task.clone()))
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<BackgroundTaskDetail> {
        self.tasks.get(id).cloned()
    }

    pub fn request_cancel(&mut self, id: &str) -> bool {
        let Some(detail) = self.tasks.get_mut(id) else {
            return false;
        };
        if detail.task.state != BackgroundTaskState::Running || !detail.task.cancellable {
            return false;
        }
        detail.task.state = BackgroundTaskState::Cancelling;
        detail.task.summary = Some("Cancellation requested; waiting for a safe checkpoint.".into());
        true
    }

    pub fn complete(
        &mut self,
        id: &str,
        state: BackgroundTaskState,
        summary: impl Into<String>,
        output: BackgroundTaskOutput,
    ) {
        if let Some(detail) = self.tasks.get_mut(id) {
            detail.task.state = state;
            detail.task.completed_at_ms = Some(now_ms());
            detail.task.progress_percent = Some(100);
            detail.task.summary = Some(summary.into());
            detail.output = Some(output);
        }
    }

    pub fn fail(&mut self, id: &str, error: AppError) {
        if let Some(detail) = self.tasks.get_mut(id) {
            detail.task.state = BackgroundTaskState::Failed;
            detail.task.completed_at_ms = Some(now_ms());
            detail.task.summary = Some(error.message.clone());
            detail.task.error = Some(error);
        }
    }

    fn trim_completed(&mut self) {
        while self.order.len() >= MAX_RETAINED_TASKS {
            let Some(id) = self.order.pop_back() else {
                break;
            };
            if self.tasks.get(&id).is_some_and(|detail| {
                matches!(
                    detail.task.state,
                    BackgroundTaskState::Running | BackgroundTaskState::Cancelling
                )
            }) {
                self.order.push_front(id);
                break;
            }
            self.tasks.remove(&id);
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle_is_stable_and_typed() {
        let mut manager = TaskManager::default();
        let task = manager.start(
            "scan-1".into(),
            BackgroundTaskKind::CleanupScan,
            "Scan",
            "/cleaner",
            true,
        );
        assert_eq!(task.state, BackgroundTaskState::Running);
        assert!(manager.request_cancel("scan-1"));
        assert_eq!(
            manager.get("scan-1").expect("task").task.state,
            BackgroundTaskState::Cancelling
        );
    }
}
