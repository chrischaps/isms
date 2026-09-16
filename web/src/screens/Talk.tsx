// Talk (GDD 12; S1.13): the Square, the channels of orgs you belong to,
// and DMs. Everything here is logged and players are told so; the
// informal economy is expected to live in the DMs.

import { useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useHome, useLexicon } from "../api/hooks";
import { useOrgs } from "../api/market";
import { useChannel, useCitizens, usePost } from "../api/civic";

export function Talk({ id, channel = "square" }: { id: number; channel?: string }) {
  const { t } = useLexicon(id);
  const home = useHome(id);
  const orgs = useOrgs(id);
  const citizens = useCitizens(id);
  const messages = useChannel(id, channel);
  const post = usePost(id, channel);
  const navigate = useNavigate();
  const [draft, setDraft] = useState("");
  const [dmTo, setDmTo] = useState("");
  const [error, setError] = useState<string | null>(null);

  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }} className="underline">
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const me = h.citizen.id;
  const handles = new Map((citizens.data?.citizens ?? []).map((z) => [z.id, z.handle]));
  const myOrgs = (orgs.data ?? []).filter(
    (o) =>
      o.i_manage ||
      o.members.includes(me) ||
      h.labor.employment.some((k) => ((k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined)?.org === o.id),
  );
  const title =
    channel === "square"
      ? "The Square"
      : channel.startsWith("org:")
        ? (myOrgs.find((o) => `org:${o.id}` === channel)?.name ?? channel)
        : `with ${handles.get(Number(channel.slice(3))) ?? `citizen ${channel.slice(3)}`}`;
  const go = (ch: string) => void navigate({ to: "/s/$id/talk/$channel", params: { id: String(id), channel: ch } });

  return (
    <div className="grid gap-8 md:grid-cols-[14rem_1fr]">
      <aside className="flex flex-col gap-4 text-sm">
        <div>
          <h3 className="text-muted text-xs uppercase tracking-wide">Channels</h3>
          <ul className="mt-1 flex flex-col gap-1">
            <li>
              <button type="button" className={channel === "square" ? "text-ink" : "text-muted underline"} onClick={() => go("square")}>
                The Square
              </button>
            </li>
            {myOrgs.map((o) => (
              <li key={o.id}>
                <button type="button" className={channel === `org:${o.id}` ? "text-ink" : "text-muted underline"} onClick={() => go(`org:${o.id}`)}>
                  {o.name}
                </button>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className="text-muted text-xs uppercase tracking-wide">Direct</h3>
          <form
            className="mt-1 flex items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              const who = (citizens.data?.citizens ?? []).find((z) => z.handle === dmTo.trim() || String(z.id) === dmTo.trim());
              if (who) go(`dm:${who.id}`);
              else setError(`No citizen called ${dmTo}.`);
            }}
          >
            <input aria-label="Message a citizen" placeholder="handle" className="border-line w-28 rounded-sm border px-1" value={dmTo} onChange={(e) => setDmTo(e.target.value)} list="citizen-handles" />
            <datalist id="citizen-handles">
              {(citizens.data?.citizens ?? []).filter((z) => z.id !== me).map((z) => (
                <option key={z.id} value={z.handle} />
              ))}
            </datalist>
            <button type="submit" className="border-line rounded-sm border px-2 py-0.5 text-xs">
              Open
            </button>
          </form>
        </div>
        <p className="text-muted text-xs">Everything said here is logged and may be published, pseudonymously, as research data. You agreed to that on joining.</p>
      </aside>

      <section className="flex flex-col gap-3">
        <h2 className="text-xl">{title}</h2>
        {error ? (
          <p className="text-bad text-sm" role="alert">
            {error}
          </p>
        ) : null}
        <div className="flex max-h-[60vh] flex-col gap-2 overflow-y-auto text-sm" data-testid="messages">
          {messages.error ? (
            <p className="text-muted">{String(messages.error)}</p>
          ) : (messages.data?.messages ?? []).length === 0 ? (
            <p className="text-muted">{messages.isPending ? "Loading." : "Nothing said yet."}</p>
          ) : (
            (messages.data?.messages ?? []).map((m) => (
              <div key={m.id} className="rule pt-1">
                <span className={m.sender === me ? "text-ink" : "text-accent"}>{m.handle}</span>
                <span className="num text-muted ml-2 text-xs">t{m.tick}</span>
                <p className="whitespace-pre-wrap">{m.body}</p>
              </div>
            ))
          )}
        </div>
        <form
          className="flex items-end gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (draft.trim() === "") return;
            setError(null);
            post.mutate(draft.trim(), {
              onSuccess: () => setDraft(""),
              onError: (err) => setError(err.message),
            });
          }}
        >
          <textarea
            aria-label="Message"
            rows={2}
            maxLength={2000}
            className="border-line flex-1 rounded-sm border px-2 py-1 text-sm"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                e.currentTarget.form?.requestSubmit();
              }
            }}
          />
          <button type="submit" disabled={post.isPending || draft.trim() === ""} className="bg-ink text-paper rounded-sm px-3 py-1 text-sm disabled:opacity-50">
            Say it
          </button>
        </form>
      </section>
    </div>
  );
}
