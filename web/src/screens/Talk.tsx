// Talk (GDD 12; S1.13; docs/style.md §10 Talk): the Square, the channels of
// orgs you belong to, DMs, and since S2.6 the floor of each proposal
// (`assembly:<pid>`), which the Assembly screen mounts `embedded` under the
// proposal. No Verdict — it is a chat. The channel list sits beside the
// messages from md and opens as a Sheet on a phone; messages are a Feed
// variant with the handle in bold. Everything here is logged and players
// are told so; the informal economy is expected to live in the DMs.

import { useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useHome, useLexicon } from "../api/hooks";
import { useOrgs } from "../api/market";
import { useChannel, useCitizens, usePost } from "../api/civic";
import { useProposals } from "../api/assembly";
import { Button, ButtonRow } from "../components/Button";
import { Card } from "../components/Card";
import { Field, Input } from "../components/Field";
import { Icon } from "../components/Icon";
import { PageHeader } from "../components/PageHeader";
import { Sheet, SheetRow } from "../components/Sheet";
import { whenOfTick } from "../lib/when";

type Channel = { key: string; label: string };

function ChannelList({ channels, current, onPick }: { channels: Channel[]; current: string; onPick: (ch: string) => void }) {
  return (
    <ul className="m-0 grid list-none gap-1 p-0">
      {channels.map((ch) => {
        const on = ch.key === current;
        return (
          <li key={ch.key}>
            <button
              type="button"
              aria-current={on ? "page" : undefined}
              className={["flex min-h-touch w-full items-center rounded-md px-3 text-left", on ? "bg-accent-soft text-accent font-bold" : "text-muted hover:bg-surface-2"].join(" ")}
              onClick={() => onPick(ch.key)}
            >
              {ch.label}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export function Talk({ id, channel = "square", embedded = false }: { id: number; channel?: string; embedded?: boolean }) {
  const { t } = useLexicon(id);
  const home = useHome(id);
  const orgs = useOrgs(id);
  const citizens = useCitizens(id);
  const messages = useChannel(id, channel);
  const post = usePost(id, channel);
  const navigate = useNavigate();
  const floor = channel.startsWith("assembly:") ? Number(channel.slice(9)) : null;
  // Only a floor's page needs the proposal's title; the list is cached for the Assembly screen anyway.
  const proposals = useProposals(id, floor !== null && !embedded);
  const [draft, setDraft] = useState("");
  const [dmTo, setDmTo] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [picker, setPicker] = useState(false);

  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }}>
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  const h = home.data!;
  const me = h.citizen.id;
  const handles = new Map((citizens.data?.citizens ?? []).map((z) => [z.id, z.handle]));
  const myOrgs = (orgs.data ?? []).filter(
    (o) =>
      o.i_manage ||
      o.members.includes(me) ||
      h.labor.employment.some((k) => ((k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined)?.org === o.id),
  );
  const floorOf = floor === null ? undefined : [...(proposals.data?.open ?? []), ...(proposals.data?.closed ?? [])].find((p) => p.id === floor);
  const title =
    channel === "square"
      ? "The Square"
      : floor !== null
        ? embedded
          ? "The floor"
          : `The floor on ${floorOf?.title ?? `proposal ${floor}`}`
        : channel.startsWith("org:")
          ? (myOrgs.find((o) => `org:${o.id}` === channel)?.name ?? channel)
          : `with ${handles.get(Number(channel.slice(3))) ?? "a citizen"}`;
  const channels: Channel[] = [{ key: "square", label: "The Square" }, ...myOrgs.map((o) => ({ key: `org:${o.id}`, label: o.name }))];
  if (channel.startsWith("dm:")) channels.push({ key: channel, label: title });
  const go = (ch: string) => {
    setPicker(false);
    void navigate({ to: "/s/$id/talk/$channel", params: { id: String(id), channel: ch } });
  };
  const openDm = () => {
    const who = (citizens.data?.citizens ?? []).find((z) => z.handle === dmTo.trim() || String(z.id) === dmTo.trim());
    if (who) go(`dm:${who.id}`);
    else setError(`No citizen called ${dmTo}.`);
  };

  const direct = (listId: string) => (
    <form
      className="grid gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        openDm();
      }}
    >
      <Field label="Message a citizen" className="max-w-none">
        <Input aria-label="Message a citizen" placeholder="handle" value={dmTo} onChange={(e) => setDmTo(e.target.value)} list={listId} />
      </Field>
      <datalist id={listId}>
        {(citizens.data?.citizens ?? [])
          .filter((z) => z.id !== me)
          .map((z) => (
            <option key={z.id} value={z.handle} />
          ))}
      </datalist>
      <Button type="submit" className="justify-self-start">
        Open
      </Button>
    </form>
  );

  const thread = (
    <>
      {floor !== null && !embedded && floorOf ? (
        <p className="text-muted mt-0 mb-3 text-sm">
          {floorOf.open ? "Open until the end of the day; " : "Decided; "}
          <Link to="/s/$id/assembly" params={{ id: String(id) }}>
            back to the assembly
          </Link>
          .
        </p>
      ) : null}
      {error ? (
        <p className="text-crit mb-3 text-sm" role="alert">
          {error}
        </p>
      ) : null}
      <div className="flex max-h-[60vh] flex-col gap-2.5 overflow-y-auto" data-testid="messages">
        {messages.error ? (
          <p className="text-muted m-0">{String(messages.error)}</p>
        ) : (messages.data?.messages ?? []).length === 0 ? (
          <p className="text-muted m-0">{messages.isPending ? "Loading." : "Nothing said yet."}</p>
        ) : (
          (messages.data?.messages ?? []).map((m) => (
            <div key={m.id} className="border-line border-b pb-2.5 last:border-b-0">
              <span className={`font-bold ${m.sender === me ? "text-ink" : "text-accent"}`}>{m.handle}</span>
              <span className="text-muted ml-2 text-sm tabular-nums">{whenOfTick(m.tick)}</span>
              <p className="m-0 whitespace-pre-wrap">{m.body}</p>
            </div>
          ))
        )}
      </div>
      <form
        className="mt-3 grid gap-2"
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
          className="border-line bg-surface text-ink focus:border-accent focus:ring-accent-soft w-full rounded-sm border px-3 py-2 focus:ring-2 focus:outline-none"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              e.currentTarget.form?.requestSubmit();
            }
          }}
        />
        <ButtonRow className="mt-0">
          <Button type="submit" variant="primary" disabled={post.isPending || draft.trim() === ""}>
            Say it
          </Button>
          <span className="text-muted text-sm">Enter sends; Shift+Enter for a new line.</span>
        </ButtonRow>
      </form>
    </>
  );

  if (embedded) {
    return (
      <div>
        <h5 className="text-muted mb-2 text-xs font-bold tracking-caps uppercase">{title}</h5>
        {thread}
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-page">
      <PageHeader
        title={title}
        meta={
          <Button className="md:hidden" onClick={() => setPicker(true)} aria-haspopup="dialog">
            <Icon name="talk" /> Channels
          </Button>
        }
      />
      <div className="grid gap-4 md:grid-cols-[14rem_minmax(0,1fr)] md:items-start">
        <div className="hidden md:grid md:gap-4">
          <Card title="Channels" icon="talk">
            <ChannelList channels={channels} current={channel} onPick={go} />
          </Card>
          <Card title="Direct">{direct("citizen-handles")}</Card>
          <p className="text-muted m-0 text-sm">Everything said here is logged and may be published, pseudonymously, as research data. You agreed to that on joining.</p>
        </div>
        <Card testId="thread">{thread}</Card>
        <p className="text-muted m-0 text-sm md:hidden">Everything said here is logged and may be published, pseudonymously, as research data. You agreed to that on joining.</p>
      </div>
      <Sheet open={picker} onClose={() => setPicker(false)} title="Channels" testId="channel-sheet">
        {channels.map((ch) => (
          <SheetRow key={ch.key}>
            <button type="button" className={`flex-1 text-left ${ch.key === channel ? "text-accent font-bold" : ""}`} onClick={() => go(ch.key)}>
              {ch.label}
            </button>
          </SheetRow>
        ))}
        <div className="mt-3">{direct("citizen-handles-sheet")}</div>
      </Sheet>
    </div>
  );
}
