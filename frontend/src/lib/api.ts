import type {
  ApiErrorPayload,
  AuthUser,
  Task,
  TaskInput,
  TaskPage,
  TaskUpdate,
} from "../types";

type StatusFilter = "all" | "pending" | "completed";

export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

let csrfToken: string | null = null;

export function setCsrfToken(token: string | null) {
  csrfToken = token;
}

async function loadCsrfToken(): Promise<string> {
  const response = await fetch("/api/auth/csrf", { credentials: "same-origin" });
  if (!response.ok) {
    if (response.status === 401) {
      window.dispatchEvent(new Event("nexora:unauthorized"));
    }
    throw new ApiError("Could not verify this request. Please sign in again.", response.status);
  }
  const body = (await response.json()) as { csrf_token: string };
  csrfToken = body.csrf_token;
  return csrfToken;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const method = (init?.method ?? "GET").toUpperCase();
  const publicAuthRequest =
    path === "/api/auth/login" || path === "/api/auth/register";
  const headers = new Headers(init?.headers);
  if (init?.body) headers.set("Content-Type", "application/json");
  if (
    ["POST", "PUT", "PATCH", "DELETE"].includes(method) &&
    !publicAuthRequest
  ) {
    headers.set("X-CSRF-Token", csrfToken ?? (await loadCsrfToken()));
  }

  const response = await fetch(path, {
    ...init,
    credentials: "same-origin",
    headers,
  });

  if (!response.ok) {
    if (response.status === 401 && !publicAuthRequest) {
      csrfToken = null;
      window.dispatchEvent(new Event("nexora:unauthorized"));
    } else if (response.status === 403) {
      csrfToken = null;
    }
    const body = (await response.json().catch(() => null)) as ApiErrorPayload | null;
    throw new ApiError(
      body?.error?.message ?? "The request could not be completed.",
      response.status,
    );
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const authApi = {
  me() {
    return request<{ user: AuthUser }>("/api/auth/me");
  },
  csrf() {
    return loadCsrfToken();
  },
  register(email: string, password: string) {
    csrfToken = null;
    return request<{ user: AuthUser }>("/api/auth/register", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    });
  },
  login(email: string, password: string) {
    csrfToken = null;
    return request<{ user: AuthUser }>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    });
  },
  async logout() {
    try {
      await request<{ logged_out: boolean }>("/api/auth/logout", {
        method: "POST",
      });
    } finally {
      csrfToken = null;
    }
  },
};

export const taskApi = {
  list(page = 1, limit = 10, status: StatusFilter = "all") {
    const query = new URLSearchParams({ page: String(page), limit: String(limit) });
    if (status !== "all") query.set("completed", String(status === "completed"));
    return request<TaskPage>(`/api/tasks?${query.toString()}`);
  },
  create(input: TaskInput) {
    return request<Task>("/api/tasks", {
      method: "POST",
      body: JSON.stringify(input),
    });
  },
  update(id: string, input: TaskUpdate) {
    return request<Task>(`/api/tasks/${encodeURIComponent(id)}`, {
      method: "PUT",
      body: JSON.stringify(input),
    });
  },
  remove(id: string) {
    return request<void>(`/api/tasks/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
  },
};
