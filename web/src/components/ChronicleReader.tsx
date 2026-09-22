// ChronicleReader (docs/style.md §7.13, §10 Society): the Chronicle read by
// day — earlier / Day N / later — as a Feed, each headline a link to its
// event where the reader may follow one. Shared by the Society screen and
// the public view.

import { Link } from "@tanstack/react-router";
import type { HeadlineView } from "../api/civic";
import { Button } from "./Button";
import { Feed } from "./Feed";
import { hourName } from "../lib/when";

/** A headline that names trouble is hot (§7.13); housekeeping stays muted. */
const HOT = /hardship|hungry|default|reject|destitut|evict|unpaid|short|collapse|missed/i;

export function ChronicleReader({
  id,
  cycle,
  setCycle,
  current,
  headlines,
  pending,
  link,
}: {
  id: number;
  cycle: number;
  setCycle: (c: number) => void;
  current: number;
  headlines: HeadlineView[];
  pending: boolean;
  link: boolean;
}) {
  return (
    <div data-testid="chronicle">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button inline variant="quiet" disabled={cycle <= 1} onClick={() => setCycle(cycle - 1)}>
          ← earlier
        </Button>
        <span className="font-bold tabular-nums">Day {cycle}</span>
        <Button inline variant="quiet" disabled={cycle >= current} onClick={() => setCycle(cycle + 1)}>
          later →
        </Button>
      </div>
      {pending ? (
        <p className="text-muted m-0">Loading.</p>
      ) : (
        <Feed
          empty="Nothing to report that day."
          items={headlines.map((h) => ({
            key: h.seq,
            hot: HOT.test(h.text),
            node: (
              <>
                <span className="text-muted mr-2 inline-block w-14 shrink-0 tabular-nums">{hourName(h.tick % 24)}</span>
                {link ? (
                  <Link to="/s/$id/events/$seq" params={{ id: String(id), seq: String(h.seq) }} className="text-inherit">
                    {h.text}
                  </Link>
                ) : (
                  <span>{h.text}</span>
                )}
              </>
            ),
          }))}
        />
      )}
    </div>
  );
}

