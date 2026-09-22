// Profile (GDD 10, TDD 10.2; S1.13): the biography line you write, the
// societies you are a citizen of with how your actions reached the engine
// (browser, API key, plan), and API keys with the notice that an agent may
// act only as a proxy for your one citizen.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useMe, useSocieties } from "../api/hooks";
import { useCreateApiKey, useRevokeApiKey, useUpdateMe, type ApiKeyCreated } from "../api/civic";
import { ThemeControl } from "../components/ThemeControl";

export function Profile() {
  const me = useMe();
  const societies = useSocieties();
  const update = useUpdateMe();
  const create = useCreateApiKey();
  const revoke = useRevokeApiKey();
  const [bio, setBio] = useState<string | null>(null);
  const [label, setLabel] = useState("agent");
  const [forSociety, setForSociety] = useState<number | null>(null);
  const [minted, setMinted] = useState<ApiKeyCreated | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (me.isPending) return <p className="text-muted">Loading.</p>;
  if (me.error) {
    return (
      <p className="text-muted">
        Not signed in.{" "}
        <Link to="/login" className="underline">
          Sign in
        </Link>
      </p>
    );
  }
  const m = me.data!;
  const names = new Map((societies.data ?? []).map((s) => [s.id, s.display]));
  const society = forSociety ?? m.citizenships[0]?.society_id ?? null;
  const biography = bio ?? (m.account.biography ?? "");

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">Profile</h2>
        <span className="text-muted text-sm">{m.account.email}</span>
      </header>
      {error ? (
        <p className="text-bad text-sm" role="alert">
          {error}
        </p>
      ) : null}

      <section className="max-w-lg">
        <h3 className="text-lg">Appearance</h3>
        <p className="text-muted mt-1 mb-2 text-xs">Light or dark, or whatever this device prefers. Remembered on this browser only.</p>
        <ThemeControl />
      </section>

      <section className="max-w-lg">
        <h3 className="text-lg">Biography</h3>
        <p className="text-muted mt-1 text-xs">One line, yours to write. It follows you across societies and epochs; nothing material does.</p>
        <form
          className="mt-2 flex items-center gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            update.mutate({ biography: biography.trim() }, { onSuccess: () => setBio(null), onError: (err) => setError(err.message) });
          }}
        >
          <input aria-label="Biography" maxLength={140} className="border-line flex-1 rounded-sm border px-2 py-1 text-sm" value={biography} onChange={(e) => setBio(e.target.value)} />
          <button type="submit" disabled={update.isPending || bio === null} className="border-line rounded-sm border px-2 py-1 text-sm disabled:opacity-50">
            Save
          </button>
        </form>
      </section>

      <section>
        <h3 className="text-lg">Societies</h3>
        {m.citizenships.length === 0 ? (
          <p className="text-muted mt-2 text-sm">
            None yet.{" "}
            <Link to="/" className="underline">
              Join one
            </Link>
            .
          </p>
        ) : (
          <table className="mt-2 w-full text-sm" data-testid="citizenships">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Society</th>
                <th className="py-1 font-normal">Handle</th>
                <th className="py-1 text-right font-normal">Honors</th>
                <th className="py-1 font-normal">How you act</th>
              </tr>
            </thead>
            <tbody>
              {m.citizenships.map((c) => {
                const share = Object.entries((c.action_share ?? {}) as Record<string, number>);
                const total = share.reduce((n, [, v]) => n + v, 0);
                return (
                  <tr key={`${c.society_id}-${c.citizen_id}`} className="rule" data-testid={`citizenship-${c.society_id}`}>
                    <td className="py-1 pr-2">
                      <Link to="/s/$id" params={{ id: String(c.society_id) }} className="underline">
                        {names.get(c.society_id) ?? `society ${c.society_id}`}
                      </Link>
                    </td>
                    <td className="py-1 pr-2">{c.handle}</td>
                    <td className="num py-1 pr-2 text-right" data-testid="honors">
                      {(c.honors ?? 0) > 0 ? c.honors : <span className="text-muted">—</span>}
                    </td>
                    <td className="py-1 text-xs" data-testid="action-share">
                      {total === 0
                        ? "no actions yet"
                        : share.map(([k, v]) => `${k.replace("_", " ")} ${((100 * v) / total).toFixed(0)}% (${v})`).join(" · ")}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        <p className="text-muted mt-2 text-xs">The share is public telemetry: the Observatory reports how much of a society is played by hand, by plan, and by agents.</p>
      </section>

      <section className="max-w-2xl">
        <h3 className="text-lg">API keys</h3>
        <p className="text-muted mt-1 text-xs">
          A key acts as one of your citizens, with everything you can do and nothing more. An agent you run with it is a proxy for that citizen, not a second player;
          every command it sends is stamped as an API action and counted above.
        </p>
        {m.api_keys.length > 0 ? (
          <table className="mt-2 w-full text-sm" data-testid="api-keys">
            <thead className="text-muted text-left text-xs uppercase tracking-wide">
              <tr>
                <th className="py-1 font-normal">Label</th>
                <th className="py-1 font-normal">Prefix</th>
                <th className="py-1 font-normal">Society</th>
                <th className="py-1 font-normal">Created</th>
                <th className="py-1 font-normal" />
              </tr>
            </thead>
            <tbody>
              {m.api_keys.map((k) => (
                <tr key={k.id} className="rule">
                  <td className="py-1 pr-2">{k.label}</td>
                  <td className="py-1 pr-2 font-mono text-xs">{k.prefix}…</td>
                  <td className="py-1 pr-2">{names.get(k.society_id) ?? `society ${k.society_id}`}</td>
                  <td className="num py-1 pr-2 text-xs">{new Date(k.created_at).toLocaleDateString()}</td>
                  <td className="py-1 text-right">
                    <button type="button" className="text-muted text-xs underline" onClick={() => revoke.mutate(k.id, { onError: (err) => setError(err.message) })}>
                      revoke
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : null}
        {society !== null ? (
          <form
            className="mt-3 flex flex-wrap items-center gap-2 text-sm"
            data-testid="new-key"
            onSubmit={(e) => {
              e.preventDefault();
              setError(null);
              create.mutate({ label: label.trim() || "agent", society_id: society }, { onSuccess: setMinted, onError: (err) => setError(err.message) });
            }}
          >
            <input aria-label="Key label" className="border-line w-32 rounded-sm border px-1" value={label} onChange={(e) => setLabel(e.target.value)} maxLength={40} />
            <span>for</span>
            <select aria-label="Key society" className="border-line rounded-sm border px-1" value={society} onChange={(e) => setForSociety(Number(e.target.value))}>
              {m.citizenships.map((c) => (
                <option key={c.society_id} value={c.society_id}>
                  {names.get(c.society_id) ?? `society ${c.society_id}`} as {c.handle}
                </option>
              ))}
            </select>
            <button type="submit" disabled={create.isPending} className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50">
              Create key
            </button>
          </form>
        ) : null}
        {minted ? (
          <div className="bg-paper-2 mt-3 rounded-sm p-3 text-sm" data-testid="minted-key">
            <p>
              Your new key, shown once. Send it as <code className="font-mono text-xs">Authorization: Bearer …</code>, or <code className="font-mono text-xs">isms login --key …</code>.
            </p>
            <code className="mt-2 block font-mono text-xs break-all" data-testid="minted-key-value">
              {minted.key}
            </code>
          </div>
        ) : null}
      </section>
    </div>
  );
}
