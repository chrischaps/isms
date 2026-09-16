// Notice-board offers as the screens read them (S1.11): an employment
// offer's terms in one object, with the pay spelled out.

import { credits, type OfferView } from "../api/client";

export function jobLine(o: OfferView) {
  const e = o.body.employment as Record<string, unknown> | undefined;
  if (!e) return null;
  const pay = e.pay as Record<string, number>;
  return {
    org: Number(e.org),
    workplace: Number(e.workplace),
    hours: Number(e.max_hours),
    places: Number(e.places),
    term: e.term_cycles == null ? "open term" : `${String(e.term_cycles)} cycles`,
    notice: Number(e.notice_cycles ?? 0),
    pay: pay.hourly !== undefined ? `${credits(pay.hourly)} cr/h` : `${credits(pay.piece_rate ?? 0)} cr/unit`,
  };
}

