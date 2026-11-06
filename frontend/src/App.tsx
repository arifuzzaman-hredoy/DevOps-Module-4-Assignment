import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import AuthScreen from "./AuthScreen";
import { ApiError, authApi, setCsrfToken, taskApi } from "./lib/api";
import type { AuthUser, Task, TaskInput } from "./types";

type StatusFilter = "all" | "pending" | "completed";
const PAGE_SIZE = 10;

interface TaskDraft {
  title: string;
  description: string;
}

const emptyDraft: TaskDraft = { title: "", description: "" };

interface TaskDashboardProps {
  user: AuthUser;
  onLogout: () => Promise<void>;
}

export default function App() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [checkingSession, setCheckingSession] = useState(true);
  const [sessionError, setSessionError] = useState("");
  const [retryCheck, setRetryCheck] = useState(0);

  useEffect(() => {
    let active = true;
    setCheckingSession(true);
    setSessionError("");

    void authApi
      .me()
      .then(async (result) => {
        const token = await authApi.csrf();
        if (active) {
          setCsrfToken(token);
          setUser(result.user);
        }
      })
      .catch((error: unknown) => {
        if (!active) return;
        setCsrfToken(null);
        if (error instanceof ApiError && error.status === 401) {
          setUser(null);
          setSessionError("");
        } else {
          setSessionError("Could not check your session. Check the service and try again.");
        }
      })
      .finally(() => {
        if (active) setCheckingSession(false);
      });

    return () => {
      active = false;
    };
  }, [retryCheck]);

  useEffect(() => {
    function handleUnauthorized() {
      setCsrfToken(null);
      setUser(null);
      setSessionError("");
      setCheckingSession(false);
    }

    window.addEventListener("nexora:unauthorized", handleUnauthorized);
    return () => window.removeEventListener("nexora:unauthorized", handleUnauthorized);
  }, []);

  useEffect(() => {
    if (checkingSession) return;
    const authPath = window.location.pathname === "/login" || window.location.pathname === "/register";
    if (!user && !authPath) {
      window.history.replaceState({}, "", "/login");
    } else if (user && authPath) {
      window.history.replaceState({}, "", "/");
    }
  }, [checkingSession, user]);

  async function logout() {
    await authApi.logout();
    setUser(null);
  }

  if (checkingSession) {
    return <main className="shell"><div className="loading-state" role="status">Checking your session…</div></main>;
  }
  if (sessionError) {
    return (
      <main className="shell">
        <div className="error-banner" role="alert">
          <span>{sessionError}</span>
          <button onClick={() => setRetryCheck((value) => value + 1)} type="button">Try again</button>
        </div>
      </main>
    );
  }
  if (!user) {
    return <AuthScreen onAuthenticated={setUser} />;
  }
  return <TaskDashboard key={user.id} onLogout={logout} user={user} />;
}

function TaskDashboard({ user, onLogout }: TaskDashboardProps) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [total, setTotal] = useState(0);
  const [totalPages, setTotalPages] = useState(0);
  const [page, setPage] = useState(1);
  const [filter, setFilter] = useState<StatusFilter>("all");
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [notice, setNotice] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Task | null>(null);
  const [draft, setDraft] = useState<TaskDraft>(emptyDraft);
  const [formError, setFormError] = useState("");
  const [saving, setSaving] = useState(false);
  const [busyTaskId, setBusyTaskId] = useState<string | null>(null);
  const [logoutError, setLogoutError] = useState("");
  const dialogRef = useRef<HTMLElement | null>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);
  const savingRef = useRef(saving);
  savingRef.current = saving;

  async function loadTasks() {
    setLoading(true);
    setLoadError("");
    try {
      const result = await taskApi.list(page, PAGE_SIZE, filter);
      setTasks(result.data);
      setTotal(result.pagination.total);
      setTotalPages(result.pagination.total_pages);
      if (result.pagination.total_pages > 0 && page > result.pagination.total_pages) {
        setPage(result.pagination.total_pages);
      }
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadTasks();
    // The function intentionally captures only the request inputs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, filter]);

  useEffect(() => {
    if (!dialogOpen) return;

    const dialog = dialogRef.current;
    const returnFocus = returnFocusRef.current;
    const previousBodyOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    dialog?.querySelector<HTMLInputElement>("#task-title")?.focus();

    function handleDialogKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !savingRef.current) {
        setDialogOpen(false);
        return;
      }

      if (event.key !== "Tab" || !dialog) return;
      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      );
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", handleDialogKeyDown);
    return () => {
      document.removeEventListener("keydown", handleDialogKeyDown);
      document.body.style.overflow = previousBodyOverflow;
      returnFocus?.focus();
    };
  }, [dialogOpen]);

  const shownTasks = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase();
    if (!needle) return tasks;
    return tasks.filter(
      (task) =>
        task.title.toLocaleLowerCase().includes(needle) ||
        (task.description ?? "").toLocaleLowerCase().includes(needle),
    );
  }, [search, tasks]);

  function openCreateDialog() {
    returnFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    setEditing(null);
    setDraft(emptyDraft);
    setFormError("");
    setDialogOpen(true);
  }

  function openEditDialog(task: Task) {
    returnFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    setEditing(task);
    setDraft({ title: task.title, description: task.description ?? "" });
    setFormError("");
    setDialogOpen(true);
  }

  async function submitTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const title = draft.title.trim();
    if (!title) {
      setFormError("Add a title before saving.");
      return;
    }
    if ([...title].length > 200) {
      setFormError("Titles can contain at most 200 characters.");
      return;
    }

    const input: TaskInput = {
      title,
      description: draft.description.trim() || null,
    };
    setSaving(true);
    setFormError("");
    try {
      if (editing) {
        await taskApi.update(editing.id, input);
        setNotice("Task updated.");
      } else {
        await taskApi.create(input);
        setNotice("Task created.");
        setPage(1);
      }
      setDialogOpen(false);
      await loadTasks();
    } catch (error) {
      setFormError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  }

  async function toggleTask(task: Task) {
    setBusyTaskId(task.id);
    setNotice("");
    try {
      await taskApi.update(task.id, { completed: !task.completed });
      setNotice(task.completed ? "Task marked as pending." : "Task completed.");
      await loadTasks();
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setBusyTaskId(null);
    }
  }

  async function deleteTask(task: Task) {
    if (!window.confirm(`Delete “${task.title}”? This cannot be undone.`)) return;
    setBusyTaskId(task.id);
    setNotice("");
    try {
      await taskApi.remove(task.id);
      setNotice("Task deleted.");
      await loadTasks();
    } catch (error) {
      setLoadError(errorMessage(error));
    } finally {
      setBusyTaskId(null);
    }
  }

  function changeFilter(nextFilter: StatusFilter) {
    setPage(1);
    setFilter(nextFilter);
  }

  async function handleLogout() {
    setLogoutError("");
    try {
      await onLogout();
    } catch (error) {
      setLogoutError(errorMessage(error));
    }
  }

  return (
    <main className="shell">
      <header className="topbar">
        <a className="brand" href="/" aria-label="Nexora home">
          <span>Nexora</span>
        </a>
        <div className="topbar-actions">
          <span className="user-email">{user.email}</span>
          <button className="button button-secondary" onClick={() => void handleLogout()} type="button">
            Log out
          </button>
          <button className="button button-primary" onClick={openCreateDialog} type="button">
            Add a task
          </button>
        </div>
      </header>

      <section className="page-heading" aria-labelledby="page-title">
        <h1 id="page-title">My tasks</h1>
        <p>Keep track of what needs to get done.</p>
      </section>

      <section className="task-panel" aria-labelledby="task-heading">
        <div className="panel-heading">
          <div>
            <h2 id="task-heading">Tasks <span className="count-pill">{total}</span></h2>
          </div>
          <label className="search-box">
            <span className="sr-only">Search tasks on this page</span>
            <input
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search tasks"
              type="search"
              value={search}
            />
          </label>
        </div>

        <div className="toolbar">
          <div aria-label="Filter tasks" className="filter-tabs" role="group">
            {(["all", "pending", "completed"] as const).map((value) => (
              <button
                aria-pressed={filter === value}
                className={filter === value ? "filter-tab active" : "filter-tab"}
                key={value}
                onClick={() => changeFilter(value)}
                type="button"
              >
                {value[0].toUpperCase() + value.slice(1)}
              </button>
            ))}
          </div>
          <span className="page-caption">
            {total === 0 ? "No tasks" : `${total} ${total === 1 ? "task" : "tasks"}`}
            {totalPages > 1 ? ` · Page ${page} of ${totalPages}` : ""}
          </span>
        </div>

        <div aria-live="polite" className="feedback">
          {notice && <p className="notice">{notice}</p>}
          {logoutError && <p className="form-error" role="alert">{logoutError}</p>}
          {loadError && (
            <div className="error-banner" role="alert">
              <span>{loadError}</span>
              <button onClick={() => void loadTasks()} type="button">Try again</button>
            </div>
          )}
        </div>

        {loading ? (
          <div aria-label="Loading tasks" className="loading-state" role="status">
            <span className="spinner" /> Loading your tasks…
          </div>
        ) : loadError ? null : shownTasks.length === 0 ? (
          <div className="empty-state">
            <h3>{search ? "No matching tasks" : filter === "all" ? "No tasks yet" : `No ${filter} tasks`}</h3>
            <p>{search ? "Try a different search on this page." : "Add a task to get started."}</p>
            {!search && filter === "all" && (
              <button className="button button-secondary" onClick={openCreateDialog} type="button">Add your first task</button>
            )}
          </div>
        ) : (
          <ul className="task-list">
            {shownTasks.map((task) => (
              <li className={task.completed ? "task-row is-complete" : "task-row"} key={task.id}>
                <input
                  aria-label={task.completed ? `Mark ${task.title} incomplete` : `Mark ${task.title} complete`}
                  className="complete-toggle"
                  checked={task.completed}
                  disabled={busyTaskId === task.id}
                  onChange={() => void toggleTask(task)}
                  type="checkbox"
                />
                <div className="task-content">
                  <div className="task-title-line">
                    <h3>{task.title}</h3>
                    <span className={task.completed ? "status-badge done" : "status-badge"}>
                      {task.completed ? "Completed" : "Pending"}
                    </span>
                  </div>
                  {task.description && <p className="task-description">{task.description}</p>}
                  <p className="task-date">
                    Created <time dateTime={task.created_at}>{new Date(task.created_at).toLocaleDateString()}</time>
                  </p>
                </div>
                <div className="task-actions">
                  <button aria-label={`Edit ${task.title}`} onClick={() => openEditDialog(task)} type="button">Edit</button>
                  <button aria-label={`Delete ${task.title}`} className="delete-action" disabled={busyTaskId === task.id} onClick={() => void deleteTask(task)} type="button">Delete</button>
                </div>
              </li>
            ))}
          </ul>
        )}

        {!loading && totalPages > 1 && (
          <nav aria-label="Task pages" className="pagination">
            <button disabled={page <= 1} onClick={() => setPage((value) => value - 1)} type="button">← Previous</button>
            <span>Page {page} of {totalPages}</span>
            <button disabled={page >= totalPages} onClick={() => setPage((value) => value + 1)} type="button">Next →</button>
          </nav>
        )}
      </section>

      <footer className="footer">
        <span>© {new Date().getFullYear()} Nexora</span>
      </footer>

      {dialogOpen && (
        <div className="dialog-backdrop" onMouseDown={(event) => {
          if (event.target === event.currentTarget && !saving) setDialogOpen(false);
        }}>
          <section aria-labelledby="dialog-title" aria-modal="true" className="task-dialog" ref={dialogRef} role="dialog">
            <button aria-label="Close dialog" className="dialog-close" disabled={saving} onClick={() => setDialogOpen(false)} type="button">×</button>
            <h2 id="dialog-title">{editing ? "Edit task" : "Create a task"}</h2>
            <p className="dialog-copy">Enter a title and an optional description.</p>
            <form onSubmit={(event) => void submitTask(event)}>
              <div className="field-heading">
                <label htmlFor="task-title">Title <span aria-hidden="true">*</span></label>
                <span className="character-count">{[...draft.title].length}/200</span>
              </div>
              <input
                aria-describedby={formError ? "task-title-help task-form-error" : "task-title-help"}
                aria-invalid={
                  formError.startsWith("Add a title") ||
                  formError.includes("at most 200 characters")
                }
                id="task-title"
                onChange={(event) => {
                  setDraft((current) => ({ ...current, title: event.target.value }));
                  setFormError("");
                }}
                placeholder="What needs to get done?"
                value={draft.title}
              />
              <p className="field-help" id="task-title-help">Titles can contain up to 200 characters.</p>
              <label htmlFor="task-description">Description <span className="optional">(optional)</span></label>
              <textarea
                id="task-description"
                onChange={(event) => {
                  setDraft((current) => ({ ...current, description: event.target.value }));
                  setFormError("");
                }}
                placeholder="Add a little more context…"
                rows={4}
                value={draft.description}
              />
              {formError && <p className="form-error" id="task-form-error" role="alert">{formError}</p>}
              <div className="dialog-actions">
                <button className="button button-secondary" disabled={saving} onClick={() => setDialogOpen(false)} type="button">Cancel</button>
                <button className="button button-primary" disabled={saving} type="submit">{saving ? "Saving…" : editing ? "Save changes" : "Create task"}</button>
              </div>
            </form>
          </section>
        </div>
      )}
    </main>
  );
}

function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.message;
  return "Could not connect to Nexora. Check the API and try again.";
}
