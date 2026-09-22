// Magic-link sign-in (TDD 10.2; docs/style.md §7.18): email, invite code the
// first time, then "check your mail". One card, one primary button. The link
// itself lands on the server, which sets the cookie and redirects here.

import { useState } from "react";
import { ApiError, api, unwrap } from "../api/client";
import { Button } from "../components/Button";
import { Card } from "../components/Card";
import { Field, Input } from "../components/Field";
import { PageHeader } from "../components/PageHeader";

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
      <div className="mx-auto max-w-md pt-6 sm:pt-12">
        <PageHeader title="Check your mail" />
        <Card icon="talk">
          <p className="m-0 text-lg">
            A sign-in link is on its way to <b className="font-mono">{email}</b>. It works once and for fifteen minutes.
          </p>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-md pt-6 sm:pt-12">
      <PageHeader title="Sign in" />
      <Card subtitle="No passwords. You get a link by mail. The first time, you need an invitation.">
        <form onSubmit={submit} className="grid gap-4">
          <Field label="Email" className="max-w-none">
            <Input type="email" required autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} />
          </Field>
          <Field label="Invite code" hint="First sign-in only." className="max-w-none">
            <Input className="font-mono" autoComplete="off" value={invite} onChange={(e) => setInvite(e.target.value)} />
          </Field>
          {error ? (
            <p className="text-crit m-0 text-sm" role="alert">
              {error}
            </p>
          ) : null}
          <Button type="submit" variant="primary" disabled={busy}>
            Send me a link
          </Button>
        </form>
      </Card>
    </div>
  );
}
