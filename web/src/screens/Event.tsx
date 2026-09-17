// One event as you may see it (TDD 5.6; S1.13): where a Chronicle headline
// and every Explain footnote lead. The payload is shown as the engine
// wrote it, filtered by the viewer rules on the server.

import { Link } from "@tanstack/react-router";
import { useEvent } from "../api/civic";
import { Num, type Explain } from "../components/Num";
import { whenOf } from "../lib/when";

export function EventScreen({ id, seq }: { id: number; seq: number }) {
  const ev = useEvent(id, seq);
  if (ev.isPending) return <p className="text-muted">Loading.</p>;
  if (ev.error) return <p className="text-bad">Could not load: {String(ev.error)}</p>;
  const e = ev.data!.event;
  const explains = ev.data!.explains as unknown as Explain[];
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">
          {e.kind.replace(/([a-z])([A-Z])/g, "$1 $2")} <span className="num text-muted text-base">#{e.seq}</span>
        </h2>
        <span className="num text-muted text-sm">
          {whenOf(e)}
        </span>
      </header>
      {explains.length > 0 ? (
        <section data-testid="event-explains">
          <h3 className="text-lg">Why</h3>
          <ul className="mt-2 flex flex-col gap-2 text-sm">
            {explains.map((x, i) => (
              <li key={i}>
                <Num value={x.rule} explain={x} />
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      <section>
        <h3 className="text-lg">As recorded</h3>
        <pre className="bg-paper-2 mt-2 overflow-x-auto rounded-sm p-3 font-mono text-xs" data-testid="event-payload">
          {JSON.stringify(e.payload, null, 2)}
        </pre>
      </section>
      <p className="text-muted text-sm">
        <Link to="/s/$id/society" params={{ id: String(id) }} className="underline">
          Back to the society
        </Link>
      </p>
    </div>
  );
}
