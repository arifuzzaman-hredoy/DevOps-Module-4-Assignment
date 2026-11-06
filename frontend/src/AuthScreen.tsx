import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { ApiError, authApi, setCsrfToken } from "./lib/api";
import type { AuthUser } from "./types";

interface AuthScreenProps {
  onAuthenticated: (user: AuthUser) => void;
}

export default function AuthScreen({ onAuthenticated }: AuthScreenProps) {
  const [registering, setRegistering] = useState(
    () => window.location.pathname === "/register",
  );
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    function syncModeWithPath() {
      setRegistering(window.location.pathname === "/register");
    }
    window.addEventListener("popstate", syncModeWithPath);
    return () => window.removeEventListener("popstate", syncModeWithPath);
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");

    if (registering && !isStrongPassword(password)) {
      setError("Use 6–128 bytes with an uppercase letter, lowercase letter, number, and symbol.");
      return;
    }

    setSubmitting(true);
    try {
      const result = registering
        ? await authApi.register(email.trim(), password)
        : await authApi.login(email.trim(), password);
      const token = await authApi.csrf();
      setCsrfToken(token);
      onAuthenticated(result.user);
    } catch (reason) {
      setCsrfToken(null);
      setError(
        reason instanceof ApiError
          ? reason.message
          : "Could not connect to Nexora. Check the service and try again.",
      );
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <main className="shell auth-shell">
      <header className="topbar">
        <a className="brand" href="/" aria-label="Nexora home">Nexora</a>
      </header>
      <section className="auth-panel" aria-labelledby="auth-heading">
        <h1 id="auth-heading">{registering ? "Create your account" : "Sign in"}</h1>
        <p className="auth-intro">
          {registering ? "Create an account to keep your tasks private." : "Sign in to manage your tasks."}
        </p>
        <form onSubmit={(event) => void submit(event)}>
          <label htmlFor="auth-email">Email</label>
          <input
            autoComplete="email"
            id="auth-email"
            maxLength={254}
            onChange={(event) => setEmail(event.target.value)}
            required
            type="email"
            value={email}
          />
          <label htmlFor="auth-password">Password</label>
          <input
            autoComplete={registering ? "new-password" : "current-password"}
            id="auth-password"
            maxLength={128}
            minLength={registering ? 6 : undefined}
            onChange={(event) => setPassword(event.target.value)}
            required
            type={showPassword ? "text" : "password"}
            value={password}
          />
          <label className="password-visibility">
            <input
              checked={showPassword}
              onChange={(event) => setShowPassword(event.target.checked)}
              type="checkbox"
            />
            Show password
          </label>
          {registering && (
            <p className="field-help">
              Use 6–128 bytes with uppercase and lowercase letters, a number, and a symbol.
            </p>
          )}
          {error && <p className="form-error" role="alert">{error}</p>}
          <button className="button button-primary auth-submit" disabled={submitting} type="submit">
            {submitting ? "Please wait…" : registering ? "Create account" : "Sign in"}
          </button>
        </form>
        <p className="auth-switch">
          {registering ? "Already have an account?" : "New to Nexora?"}{" "}
          <button
            disabled={submitting}
            onClick={() => {
              const nextRegistering = !registering;
              setRegistering(nextRegistering);
              window.history.pushState({}, "", nextRegistering ? "/register" : "/login");
              setError("");
            }}
            type="button"
          >
            {registering ? "Sign in" : "Create an account"}
          </button>
        </p>
      </section>
    </main>
  );
}

function isStrongPassword(password: string): boolean {
  const byteLength = new TextEncoder().encode(password).length;
  return (
    byteLength >= 6 &&
    byteLength <= 128 &&
    /[a-z]/.test(password) &&
    /[A-Z]/.test(password) &&
    /\d/.test(password) &&
    /[^a-zA-Z0-9]/.test(password)
  );
}
