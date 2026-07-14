import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, CircleX, LoaderCircle, ListTodo, X } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { taskApi } from "../lib/tasks-ipc";
import type { BackgroundTask } from "../types/tasks";

export function TaskCenter() {
  const [open, setOpen] = useState(false);
  const client = useQueryClient();
  const tasks = useQuery({
    queryKey: ["backgroundTasks"],
    queryFn: taskApi.list,
    refetchInterval: (query) =>
      (query.state.data ?? []).some((task) => isActive(task)) ? 750 : 3_000,
  });
  const cancel = useMutation({
    mutationFn: taskApi.cancel,
    onSuccess: () => void client.invalidateQueries({ queryKey: ["backgroundTasks"] }),
  });
  const activeCount = (tasks.data ?? []).filter(isActive).length;
  const visible = (tasks.data ?? []).slice(0, 6);

  return (
    <div className="task-center">
      <button
        className="task-center-trigger"
        type="button"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
      >
        {activeCount ? <LoaderCircle className="spin" size={15} /> : <ListTodo size={15} />}
        <span>{activeCount ? `${activeCount} active` : "Tasks"}</span>
      </button>
      {open && (
        <section className="task-center-panel" aria-label="Background tasks">
          <header>
            <div>
              <strong>Background tasks</strong>
              <span>Continue while you move between views.</span>
            </div>
            <button className="icon-button" type="button" onClick={() => setOpen(false)}>
              <X size={15} />
            </button>
          </header>
          <div className="task-center-list">
            {visible.map((task) => (
              <article key={task.id} className={`task-row task-${task.state}`}>
                {taskIcon(task)}
                <span>
                  <Link to={task.route} onClick={() => setOpen(false)}>
                    {task.label}
                  </Link>
                  <small>{task.summary ?? stateLabel(task)}</small>
                </span>
                {task.cancellable && task.state === "running" && (
                  <button
                    className="button button-secondary compact"
                    type="button"
                    disabled={cancel.isPending}
                    onClick={() => cancel.mutate(task.id)}
                  >
                    Cancel
                  </button>
                )}
              </article>
            ))}
            {!tasks.isLoading && visible.length === 0 && (
              <div className="empty-inline">No background tasks in this app session.</div>
            )}
          </div>
        </section>
      )}
    </div>
  );
}

function isActive(task: BackgroundTask) {
  return task.state === "running" || task.state === "cancelling";
}

function taskIcon(task: BackgroundTask) {
  if (isActive(task)) return <LoaderCircle className="spin" size={16} />;
  if (task.state === "succeeded") return <CheckCircle2 size={16} />;
  return <CircleX size={16} />;
}

function stateLabel(task: BackgroundTask) {
  if (task.state === "running") return "Running in Mr Manager";
  if (task.state === "cancelling") return "Cancelling at a safe checkpoint";
  if (task.state === "succeeded") return "Completed";
  if (task.state === "cancelled") return "Cancelled";
  return task.error?.message ?? "Failed";
}
