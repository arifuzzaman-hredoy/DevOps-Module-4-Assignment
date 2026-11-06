import { ApiError, authApi, setCsrfToken, taskApi } from "./api";

describe("task API client", () => {
  beforeEach(() => {
    setCsrfToken(null);
    vi.stubGlobal("fetch", vi.fn());
  });

  it("uses a relative URL and query filters for paginated task reads", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ data: [], pagination: {} }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await taskApi.list(2, 20, "pending");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/tasks?page=2&limit=20&completed=false",
      expect.objectContaining({ credentials: "same-origin" }),
    );
  });

  it("converts API error envelopes to ApiError", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ error: { code: "TASK_NOT_FOUND", message: "Task not found" } }), {
          status: 404,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    );

    await expect(taskApi.list()).rejects.toMatchObject({
      message: "Task not found",
      status: 404,
    } satisfies Partial<ApiError>);
  });

  it("sends a task payload as JSON", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ id: "task-1" }), { status: 201 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    setCsrfToken("session-csrf-token");

    await taskApi.create({ title: "A task", description: null });

    const [, options] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(options).toMatchObject({
      method: "POST",
      body: JSON.stringify({ title: "A task", description: null }),
      credentials: "same-origin",
    });
    expect(new Headers(options.headers).get("Content-Type")).toBe("application/json");
    expect(new Headers(options.headers).get("X-CSRF-Token")).toBe("session-csrf-token");
  });

  it("retrieves a CSRF token before an authenticated mutation when needed", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ csrf_token: "fresh-token" }), { status: 200 }),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ id: "task-1" }), { status: 201 }));
    vi.stubGlobal("fetch", fetchMock);

    await taskApi.create({ title: "A task" });

    expect(fetchMock.mock.calls[0][0]).toBe("/api/auth/csrf");
    const options = fetchMock.mock.calls[1][1] as RequestInit;
    expect(new Headers(options.headers).get("X-CSRF-Token")).toBe("fresh-token");
  });

  it("clears frontend auth state on an API 401", async () => {
    const unauthorized = vi.fn();
    window.addEventListener("nexora:unauthorized", unauthorized);
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ error: { message: "Authentication is required" } }), {
          status: 401,
        }),
      ),
    );

    await expect(authApi.me()).rejects.toMatchObject({ status: 401 });
    expect(unauthorized).toHaveBeenCalledOnce();
    window.removeEventListener("nexora:unauthorized", unauthorized);
  });
});
