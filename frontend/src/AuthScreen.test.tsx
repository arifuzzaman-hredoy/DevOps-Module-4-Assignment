import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import AuthScreen from "./AuthScreen";
import { authApi } from "./lib/api";
import type { AuthUser } from "./types";

vi.mock("./lib/api", () => ({
  ApiError: class ApiError extends Error {
    constructor(message: string, public readonly status = 500) {
      super(message);
    }
  },
  authApi: {
    register: vi.fn(),
    login: vi.fn(),
    csrf: vi.fn(),
  },
  setCsrfToken: vi.fn(),
}));

const userProfile: AuthUser = {
  id: "user-1",
  email: "person@example.com",
  created_at: "2026-01-01T10:00:00Z",
};

const register = vi.mocked(authApi.register);
const login = vi.mocked(authApi.login);
const csrf = vi.mocked(authApi.csrf);

beforeEach(() => {
  window.history.replaceState({}, "", "/login");
  vi.clearAllMocks();
  register.mockResolvedValue({ user: userProfile });
  login.mockResolvedValue({ user: userProfile });
  csrf.mockResolvedValue("csrf-test-token");
});

describe("authentication form", () => {
  it("submits valid login credentials and establishes csrf state", async () => {
    const user = userEvent.setup();
    const onAuthenticated = vi.fn();
    render(<AuthScreen onAuthenticated={onAuthenticated} />);
    await user.type(screen.getByLabelText("Email"), "person@example.com");
    await user.type(screen.getByLabelText("Password"), "StrongPassword123!");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => expect(login).toHaveBeenCalledWith("person@example.com", "StrongPassword123!"));
    expect(csrf).toHaveBeenCalled();
    expect(onAuthenticated).toHaveBeenCalledWith(userProfile);
  });

  it("requires the documented password policy on registration", async () => {
    const user = userEvent.setup();
    render(<AuthScreen onAuthenticated={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Create an account" }));
    await user.type(screen.getByLabelText("Email"), "new@example.com");
    await user.type(screen.getByLabelText("Password"), "weak");
    await user.click(screen.getByRole("button", { name: "Create account" }));

    expect(screen.getByRole("alert").textContent).toContain("6–128 bytes");
    expect(register).not.toHaveBeenCalled();
  });

  it("registers successfully and allows password visibility toggling", async () => {
    const user = userEvent.setup();
    const onAuthenticated = vi.fn();
    render(<AuthScreen onAuthenticated={onAuthenticated} />);
    await user.click(screen.getByRole("button", { name: "Create an account" }));
    expect(window.location.pathname).toBe("/register");
    await user.type(screen.getByLabelText("Email"), "new@example.com");
    const password = screen.getByLabelText("Password");
    await user.type(password, "StrongPassword123!");
    await user.click(screen.getByLabelText("Show password"));
    expect(password).toHaveProperty("type", "text");
    fireEvent.click(screen.getByLabelText("Show password"));
    expect(password).toHaveProperty("type", "password");
    await user.click(screen.getByRole("button", { name: "Create account" }));

    await waitFor(() => expect(register).toHaveBeenCalledWith("new@example.com", "StrongPassword123!"));
    expect(onAuthenticated).toHaveBeenCalledWith(userProfile);
  });

  it("accepts a six-byte password that meets the character requirements", async () => {
    const user = userEvent.setup();
    render(<AuthScreen onAuthenticated={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Create an account" }));
    await user.type(screen.getByLabelText("Email"), "new@example.com");
    await user.type(screen.getByLabelText("Password"), "Aa1!bc");
    await user.click(screen.getByRole("button", { name: "Create account" }));

    await waitFor(() => expect(register).toHaveBeenCalledWith("new@example.com", "Aa1!bc"));
  });

  it("opens registration directly from the /register route", () => {
    window.history.replaceState({}, "", "/register");
    render(<AuthScreen onAuthenticated={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Create your account" })).toBeTruthy();
  });

  it("shows a generic server authentication error", async () => {
    const user = userEvent.setup();
    login.mockRejectedValue(new Error("network unavailable"));
    render(<AuthScreen onAuthenticated={vi.fn()} />);
    await user.type(screen.getByLabelText("Email"), "person@example.com");
    await user.type(screen.getByLabelText("Password"), "WrongPassword123!");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toContain("Could not connect");
  });
});
