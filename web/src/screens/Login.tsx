// Magic-link sign-in (TDD 10.2): email, invite code the first time, then
// "check your mail". The link itself lands on the server, which sets the
// cookie and redirects here.

import { useState } from "react";
import { ApiError, api, unwrap } from "../api/client";

export function Login() {
  const [email, setEmail] = useState("");
  const [invite, setInvite] = useState("");
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      unwrap(
        await api.POST("/auth/magic-link", {
          body: { email, invite_code: invite.trim() || undefined },
        }),
      );
      setSent(true);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  if (sent) {
    return (
      <section className="max-w-md">
        <h1 className="text-2xl">Check your mail</h1>
        <p className="mt-3">
          A sign-in link is on its way to <span className="font-mono">{email}</span>. It works once and
          for fifteen minutes.
        </p>
      </section>
    );
  }

  return (
    <section className="max-w-md">
      <h1 className="text-2xl">Sign in</h1>
      <p className="text-muted mt-2 text-sm">
        No passwords. You get a link by mail. The first time, you need an invitation.
      </p>
      <form onSubmit={submit} className="mt-6 flex flex-col gap-4">
        <label className="flex flex-col gap-1 text-sm">
          Email
          <input
            type="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="border-line bg-paper-2 rounded-sm border px-2 py-1"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          Invite code <span className="text-muted">(first sign-in only)</span>
          <input
            value={invite}
            onChange={(e) => setInvite(e.target.value)}
            className="border-line bg-paper-2 rounded-sm border px-2 py-1 font-mono"
          />
        </label>
        {error ? <p className="text-bad text-sm">{error}</p> : null}
        <button
          type="submit"
          disabled={busy}
          className="bg-ink text-paper self-start rounded-sm px-3 py-1 disabled:opacity-50"
        >
          Send me a link
        </button>
      </form>
    </section>
  );
}
