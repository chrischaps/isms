// One event as you may see it (TDD 5.6; S1.13; docs/style.md §2 layer 5):
// where a Chronicle headline and every Explain footnote lead. No Verdict —
// this is the explanation itself. The rules that produced it open as Nums;
// the payload is shown as the engine wrote it, filtered by the viewer rules
// on the server.

import { Link } from "@tanstack/react-router";
import { useEvent } from "../api/civic";
import { Card, Stack } from "../components/Card";
import { FactList } from "../components/FactList";
import { Num, showValue, type Explain } from "../components/Num";
import { PageHeader } from "../components/PageHeader";
import { whenOf } from "../lib/when";

export function EventScreen({ id, seq }: { id: number; seq: number }) {
  const ev = useEvent(id, seq);
  if (ev.isPending) return <p className="text-muted">Loading.</p>;
  if (ev.error) return <p className="text-crit">Could not load: {String(ev.error)}</p>;
  const e = ev.data!.event;
  const explains = ev.data!.explains as unknown as Explain[];
  return (
    <div className="mx-auto max-w-page">
      <PageHeader
        title={
          <>
            {e.kind.replace(/([a-z])([A-Z])/g, "$1 $2")} <span className="text-muted text-lg font-normal tabular-nums">#{e.seq}</span>
          </>
        }
        meta={
          <>
            <span className="tabular-nums">{whenOf(e)}</span>
            <Link to="/s/$id/society" params={{ id: String(id) }}>
              Back to the society
            </Link>
          </>
        }
      />
      <Stack>
        {explains.length > 0 ? (
          <Card title="Why" icon="page" testId="event-explains" subtitle="The rules that produced it; each opens its inputs.">
            <FactList items={explains.map((x, i) => ({ key: String(i), label: x.rule, value: <Num value={showValue(x.result)} explain={x} /> }))} />
          </Card>
        ) : null}
        <Card title="As recorded" icon="ledger" subtitle="The payload as the engine wrote it.">
          <pre className="bg-surface-2 m-0 rounded-sm p-3 font-mono text-xs break-all whitespace-pre-wrap" data-testid="event-payload">
            {JSON.stringify(e.payload, null, 2)}
          </pre>
        </Card>
      </Stack>
    </div>
  );
}
