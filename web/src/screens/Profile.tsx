// Profile (GDD 10, TDD 10.2; S1.13; docs/style.md §10): the appearance
// switch, the biography line you write, the societies you are a citizen of
// with how your actions reached the engine (browser, API key, plan), and API
// keys under a disclosure with the notice that an agent may act only as a
// proxy for your one citizen. Plain fact lists and forms; no Verdict.

import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useMe, useSocieties } from "../api/hooks";
import { useCreateApiKey, useRevokeApiKey, useUpdateMe, type ApiKeyCreated } from "../api/civic";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile } from "../components/Card";
import { Field, Input, Select } from "../components/Field";
import { TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { More } from "../components/More";
import { PageHeader } from "../components/PageHeader";
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
        Not signed in. <Link to="/login">Sign in</Link>
      </p>
    );
  }
  const m = me.data!;
  const names = new Map((societies.data ?? []).map((s) => [s.id, s.display]));
  const society = forSociety ?? m.citizenships[0]?.society_id ?? null;
  const biography = bio ?? (m.account.biography ?? "");

  return (
    <div>
      <PageHeader
        title="Profile"
        meta={
          <>
            <span>{m.account.email}</span>
            <Link to="/">Societies</Link>
            {m.account.operator ? <Link to="/admin">Operator</Link> : null}
          </>
        }
      />
      {error ? (
        <p className="text-crit mt-0 mb-4 text-sm" role="alert">
          {error}
        </p>
      ) : null}
      <Stack>
        <Card title="Appearance" icon="spark" subtitle="Light or dark, or whatever this device prefers. Remembered on this browser only.">
          <ThemeControl />
        </Card>

        <Card title="Biography" icon="person" subtitle="One line, yours to write. It follows you across societies and epochs; nothing material does.">
          <form
            className="grid gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              update.mutate({ biography: biography.trim() }, { onSuccess: () => setBio(null), onError: (err) => setError(err.message) });
            }}
          >
            <Field label="Biography" className="max-w-none">
              <Input aria-label="Biography" maxLength={140} value={biography} onChange={(e) => setBio(e.target.value)} />
            </Field>
            <ButtonRow>
              <Button type="submit" variant="primary" disabled={update.isPending || bio === null}>
                Save
              </Button>
              {update.isSuccess && bio === null ? <span className="text-good text-sm">Kept.</span> : null}
            </ButtonRow>
          </form>
        </Card>

        <Card title="Societies" icon="globe" subtitle="Where you are a citizen, and how your actions have reached the engine.">
          {m.citizenships.length === 0 ? (
            <p className="text-muted m-0">
              None yet. <Link to="/">Join one</Link>.
            </p>
          ) : (
            <table className="w-full border-collapse text-[15px]" data-testid="citizenships">
              <thead>
                <tr>
                  <th className={TH}>Society</th>
                  <th className={`${TH} hidden md:table-cell`}>Handle</th>
                  <th className={TH_NUM}>Honors</th>
                  <th className={`${TH} hidden md:table-cell`}>How you act</th>
                </tr>
              </thead>
              <tbody>
                {m.citizenships.map((c) => {
                  const share = Object.entries((c.action_share ?? {}) as Record<string, number>);
                  const total = share.reduce((n, [, v]) => n + v, 0);
                  const how = total === 0 ? "no actions yet" : share.map(([k, v]) => `${k.replace("_", " ")} ${((100 * v) / total).toFixed(0)} % (${v})`).join(" · ");
                  return (
                    <tr key={`${c.society_id}-${c.citizen_id}`} className="hover:bg-surface-2" data-testid={`citizenship-${c.society_id}`}>
                      <td className={TD}>
                        <Link to="/s/$id" params={{ id: String(c.society_id) }}>
                          {names.get(c.society_id) ?? `society ${c.society_id}`}
                        </Link>
                        <span className="text-muted block text-sm md:hidden">
                          as {c.handle} · {how}
                        </span>
                      </td>
                      <td className={`${TD} hidden md:table-cell`}>{c.handle}</td>
                      <td className={TD_NUM} data-testid="honors">
                        {(c.honors ?? 0) > 0 ? c.honors : <span className="text-muted">—</span>}
                      </td>
                      <td className={`${TD} hidden text-sm md:table-cell`} data-testid="action-share">
                        {how}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
          <p className="text-muted mt-3 mb-0 text-sm">The share is public telemetry: the Observatory reports how much of a society is played by hand, by plan, and by agents.</p>
        </Card>

        <Card
          title="API keys"
          icon="gear"
          subtitle="A key acts as one of your citizens, with everything you can do and nothing more. An agent you run with it is a proxy for that citizen, not a second player; every command it sends is stamped as an API action and counted above."
        >
          {m.api_keys.length > 0 ? (
            <table className="w-full border-collapse text-[15px]" data-testid="api-keys">
              <thead>
                <tr>
                  <th className={TH}>Label</th>
                  <th className={`${TH} hidden md:table-cell`}>Prefix</th>
                  <th className={TH}>Society</th>
                  <th className={`${TH} hidden md:table-cell`}>Created</th>
                  <th className={TH} />
                </tr>
              </thead>
              <tbody>
                {m.api_keys.map((k) => (
                  <tr key={k.id} className="hover:bg-surface-2">
                    <td className={TD}>
                      {k.label}
                      <span className="text-muted block font-mono text-xs md:hidden">
                        {k.prefix}… · {new Date(k.created_at).toLocaleDateString()}
                      </span>
                    </td>
                    <td className={`${TD} hidden font-mono text-xs md:table-cell`}>{k.prefix}…</td>
                    <td className={TD}>{names.get(k.society_id) ?? `society ${k.society_id}`}</td>
                    <td className={`${TD} hidden text-sm md:table-cell`}>{new Date(k.created_at).toLocaleDateString()}</td>
                    <td className={`${TD} text-right`}>
                      <Button variant="quiet" inline onClick={() => revoke.mutate(k.id, { onError: (err) => setError(err.message) })}>
                        revoke
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="text-muted m-0">No key yet.</p>
          )}
          {society !== null ? (
            <More summary="Create a key" testId="new-key-more">
              <form
                className="grid gap-3 sm:grid-cols-2"
                data-testid="new-key"
                onSubmit={(e) => {
                  e.preventDefault();
                  setError(null);
                  create.mutate({ label: label.trim() || "agent", society_id: society }, { onSuccess: setMinted, onError: (err) => setError(err.message) });
                }}
              >
                <Field label="Label">
                  <Input aria-label="Key label" value={label} onChange={(e) => setLabel(e.target.value)} maxLength={40} />
                </Field>
                <Field label="Acts as">
                  <Select aria-label="Key society" value={society} onChange={(e) => setForSociety(Number(e.target.value))}>
                    {m.citizenships.map((c) => (
                      <option key={c.society_id} value={c.society_id}>
                        {names.get(c.society_id) ?? `society ${c.society_id}`} as {c.handle}
                      </option>
                    ))}
                  </Select>
                </Field>
                <ButtonRow className="sm:col-span-2">
                  <Button type="submit" variant="primary" disabled={create.isPending}>
                    Create key
                  </Button>
                </ButtonRow>
              </form>
              {minted ? (
                <Tile className="mt-3 text-sm" testId="minted-key">
                  <p className="mt-0 mb-2">
                    Your new key, shown once. Send it as <code className="font-mono text-xs">Authorization: Bearer …</code>, or <code className="font-mono text-xs">isms login --key …</code>.
                  </p>
                  <code className="block font-mono text-xs break-all" data-testid="minted-key-value">
                    {minted.key}
                  </code>
                </Tile>
              ) : null}
            </More>
          ) : null}
        </Card>
      </Stack>
    </div>
  );
}
