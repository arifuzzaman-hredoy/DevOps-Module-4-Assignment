import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { ApiError, authApi, taskApi } from "./lib/api";
import type { AuthUser, Task, TaskPage } from "./types";

vi.mock("./lib/api", () => ({
  ApiError: class ApiError extends Error {
    constructor(message: string, public readonly status = 500) {
      super(message);
    }
  },
  authApi: {
    me: vi.fn(),
    csrf: vi.fn(),
    register: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
  },
  setCsrfToken: vi.fn(),
  taskApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    remove: vi.fn(),
  },
}));

const userProfile: AuthUser = {
  id: "user-1",
  email: "person@example.com",
  created_at: "2026-01-01T10:00:00Z",
};

const task: Task = {
  id: "task-1",
  title: "Learn Rust",
  description: "Practice ownership",
  completed: false,
  created_at: "2026-01-01T10:00:00Z",
  updated_at: "2026-01-01T10:00:00Z",
};

function page(tasks: Task[] = [task]): TaskPage {
  return {
    data: tasks,
    pagination: { page: 1, limit: 10, total: tasks.length, total_pages: 1 },
  };
}

const list = vi.mocked(taskApi.list);
const me = vi.mocked(authApi.me);
const csrf = vi.mocked(authApi.csrf);
const logout = vi.mocked(authApi.logout);
const create = vi.mocked(taskApi.create);
const update = vi.mocked(taskApi.update);
const remove = vi.mocked(taskApi.remove);

beforeEach(() => {
  window.history.replaceState({}, "", "/");
  vi.clearAllMocks();
  me.mockResolvedValue({ user: userProfile });
  csrf.mockResolvedValue("csrf-test-token");
  logout.mockResolvedValue(undefined);
  list.mockResolvedValue(page());
  create.mockResolvedValue(task);
  update.mockResolvedValue({ ...task, completed: true });
  remove.mockResolvedValue(undefined);
  vi.stubGlobal("confirm", vi.fn(() => true));
});

describe("Nexora task dashboard", () => {
  it("redirects unauthenticated visitors to the sign-in route", async () => {
    me.mockRejectedValueOnce(new ApiError("Authentication is required", 401));
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeTruthy();
    await waitFor(() => expect(window.location.pathname).toBe("/login"));
  });

  it("renders tasks and requests the first page", async () => {
    render(<App />);

    expect(await screen.findByText("Learn Rust")).toBeTruthy();
    expect(screen.getByText("person@example.com")).toBeTruthy();
    expect(list).toHaveBeenCalledWith(1, 10, "all");
    expect(screen.getAllByText("Practice ownership")).toHaveLength(1);
  });

  it("shows a loading state while the task list is pending", async () => {
    let resolveList!: (result: TaskPage) => void;
    list.mockImplementationOnce(() => new Promise((resolve) => {
      resolveList = resolve;
    }));
    render(<App />);

    expect(await screen.findByRole("status", { name: "Loading tasks" })).toBeTruthy();
    resolveList(page());
    expect(await screen.findByText("Learn Rust")).toBeTruthy();
  });

  it("shows the empty state and first-task action", async () => {
    list.mockResolvedValueOnce(page([]));
    render(<App />);

    expect(await screen.findByText("No tasks yet")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Add your first task" })).toBeTruthy();
  });

  it("shows a task-list error and retries the request", async () => {
    const user = userEvent.setup();
    list
      .mockRejectedValueOnce(new ApiError("Service unavailable", 503))
      .mockResolvedValueOnce(page());
    render(<App />);

    expect((await screen.findByRole("alert")).textContent).toContain("Service unavailable");
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText("Learn Rust")).toBeTruthy();
  });

  it("validates whitespace-only titles before creating a task", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    await user.type(screen.getByLabelText(/title/i), "   ");
    await user.click(screen.getByRole("button", { name: "Create task" }));

    expect(screen.getByRole("alert").textContent).toContain("Add a title");
    expect(create).not.toHaveBeenCalled();
  });

  it("shows a visible validation message for an empty title", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    await user.click(screen.getByRole("button", { name: "Create task" }));

    expect(screen.getByRole("alert").textContent).toContain("Add a title");
    expect(screen.getByLabelText(/title/i).getAttribute("aria-invalid")).toBe("true");
    expect(create).not.toHaveBeenCalled();
  });

  it("counts Unicode title characters and accepts the 200-character limit", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    fireEvent.change(screen.getByLabelText(/title/i), {
      target: { value: "ক".repeat(200) },
    });

    expect(screen.getByText("200/200")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Create task" }));
    await waitFor(() => expect(create).toHaveBeenCalled());
  });

  it("rejects titles over 200 characters without truncating them", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    const title = "x".repeat(201);
    fireEvent.change(screen.getByLabelText(/title/i), { target: { value: title } });
    await user.click(screen.getByRole("button", { name: "Create task" }));

    expect(screen.getByLabelText(/title/i)).toHaveProperty("value", title);
    expect(screen.getByRole("alert").textContent).toContain("at most 200 characters");
    expect(create).not.toHaveBeenCalled();
  });

  it("closes the task dialog with Escape and restores focus", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    const addButton = screen.getByRole("button", { name: /add a task/i });
    await user.click(addButton);
    expect(screen.getByRole("dialog")).toBeTruthy();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(addButton);
  });

  it("keeps keyboard focus inside the task dialog", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    const closeButton = screen.getByRole("button", { name: "Close dialog" });
    const submitButton = screen.getByRole("button", { name: "Create task" });

    await user.tab({ shift: true });
    expect(document.activeElement).toBe(closeButton);
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(submitButton);
    await user.tab();
    expect(document.activeElement).toBe(closeButton);
  });

  it("creates a task and refreshes the list", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: /add a task/i }));
    await user.type(screen.getByLabelText(/title/i), "Write tests");
    await user.type(screen.getByLabelText(/description/i), "Cover the API");
    await user.click(screen.getByRole("button", { name: "Create task" }));

    await waitFor(() =>
      expect(create).toHaveBeenCalledWith({
        title: "Write tests",
        description: "Cover the API",
      }),
    );
    expect(list).toHaveBeenCalledTimes(2);
    expect(await screen.findByText("Task created.")).toBeTruthy();
  });

  it("edits a task and submits the updated fields", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: "Edit Learn Rust" }));

    expect(screen.getByRole("heading", { name: "Edit task" })).toBeTruthy();
    const title = screen.getByLabelText(/title/i);
    await user.clear(title);
    await user.type(title, "Learn Rust well");
    const description = screen.getByLabelText(/description/i);
    await user.clear(description);
    await user.type(description, "Practice every day");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith("task-1", {
        title: "Learn Rust well",
        description: "Practice every day",
      }),
    );
    expect(await screen.findByText("Task updated.")).toBeTruthy();
  });

  it("marks a task complete and sends the explicit completed value", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("checkbox", { name: "Mark Learn Rust complete" }));

    await waitFor(() => expect(update).toHaveBeenCalledWith("task-1", { completed: true }));
    expect(list).toHaveBeenCalledTimes(2);
  });

  it("filters tasks by status through the API", async () => {
    const user = userEvent.setup();
    list.mockResolvedValueOnce(page()).mockResolvedValueOnce(page([]));
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: "Completed" }));

    await waitFor(() => expect(list).toHaveBeenLastCalledWith(1, 10, "completed"));
    expect(screen.getByText("No completed tasks")).toBeTruthy();
  });

  it("searches the currently loaded page", async () => {
    render(<App />);
    await screen.findByText("Learn Rust");
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "missing" } });

    expect(screen.getByText("No matching tasks")).toBeTruthy();
    expect(screen.queryByText("Learn Rust")).toBeNull();
  });

  it("asks for confirmation before deleting a task", async () => {
    const user = userEvent.setup();
    render(<App />);
    const row = await screen.findByText("Learn Rust");
    await user.click(within(row.closest("li")!).getByRole("button", { name: "Delete Learn Rust" }));

    expect(confirm).toHaveBeenCalled();
    expect(remove).toHaveBeenCalledWith("task-1");
  });

  it("logs out and returns to the sign-in form", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Learn Rust");
    await user.click(screen.getByRole("button", { name: "Log out" }));

    await waitFor(() => expect(logout).toHaveBeenCalled());
    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeTruthy();
  });
});
