// The ballot builder (GDD 15 "role tooling", TDD 4 roles/; S2.6): one typed
// form per proposal kind the constitution enables. The form knows what a
// kind needs and nothing about whether the assembly will have it: the engine
// refuses, and the refusal is shown in the mover's own words with the ids
// named. S2.8's rationing form mounts this with `only={["policy_change"]}`.

import { useState } from "react";
import { type ProposalKind, usePropose } from "../../api/assembly";
import type { CapabilitiesView } from "../../api/client";
import { fieldName, kindTitle } from "../../lib/policy";

export type Citizen = { id: number; handle: string };
export type Office = { kind: string; holders: { citizen: number; handle: string }[] };

type FieldKind = "select" | "hours" | "split";
type FieldSpec = {
  key: string;
  label: string;
  hint: string;
  kind: FieldKind;
  options?: string[];
  note?: string;
  show: (c: CapabilitiesView) => boolean;
};

/** The engine's `PolicyPatch` fields, each shown only where the society has the thing it governs. */
export const POLICY_FIELDS: FieldSpec[] = [
  {
    key: "monitoring",
    label: "Monitoring",
    hint: "how closely each worker's output is attributed to them",
    kind: "select",
    options: ["inherit", "high", "medium", "low"],
    show: () => true,
  },
  {
    key: "work_norm_hours",
    label: "Work norm",
    hint: "the hours a day the norm asks of everyone",
    kind: "hours",
    show: (c) => c.labor === "norm",
  },
  {
    key: "rationing",
    label: "Rationing rule",
    hint: "how the Store serves a good that runs short",
    kind: "select",
    options: ["need_first", "equal_shortfall", "lottery"],
    note: "only a coordinator may move this",
    show: (c) => c.common_store,
  },
  {
    key: "materials_split",
    label: "Materials split",
    hint: "where the Materials go, in percent: Wares, Machines, Dwellings; the three must make 100",
    kind: "split",
    show: (c) => c.common_store || c.administered_prices,
  },
];

type Split = { wares: number; machines: number; dwellings: number };

/** Percentages typed by a person to the engine's fractions. */
export function splitToFractions(s: Split): Split {
  return { wares: s.wares / 100, machines: s.machines / 100, dwellings: s.dwellings / 100 };
}

/** The proposal kinds a citizen can move from here: an election is not moved, it is stood for (Q125). */
export function movableKinds(kinds: string[], only?: string[]): string[] {
  return kinds.filter((k) => k !== "election" && k !== "disbursement" && k !== "admission" && (!only || only.includes(k)));
}

export function BallotBuilder({
  id,
  caps,
  citizens,
  offices,
  only,
  fields: onlyFields,
  onMoved,
}: {
  id: number;
  caps: CapabilitiesView;
  citizens: Citizen[];
  offices: Office[];
  /** Restrict the builder to these kinds (S2.8 mounts it for the rationing rule alone). */
  only?: string[];
  /** Restrict a policy change to these fields (the Coordinator workspace: `["rationing"]`). */
  fields?: string[];
  onMoved?: (title: string) => void;
}) {
  const kinds = movableKinds(caps.proposal_kinds ?? [], only);
  const propose = usePropose(id);
  const [kind, setKind] = useState<string>(kinds[0] ?? "");
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [patch, setPatch] = useState<Record<string, unknown>>({});
  const [split, setSplit] = useState<Split>({ wares: 50, machines: 30, dwellings: 20 });
  const [honoree, setHonoree] = useState<number | "">("");
  const [recallOffice, setRecallOffice] = useState<string>("");
  const [recallWho, setRecallWho] = useState<number | "">("");
  const [error, setError] = useState<string | null>(null);
  const [moved, setMoved] = useState<string | null>(null);

  if (kinds.length === 0) return null;
  const current = kinds.includes(kind) ? kind : kinds[0]!;
  const fields = POLICY_FIELDS.filter((f) => f.show(caps) && (!onlyFields || onlyFields.includes(f.key)));
  const heldOffices = offices.filter((o) => o.holders.length > 0);
  const recallHolders = heldOffices.find((o) => o.kind === recallOffice)?.holders ?? [];
  const splitSum = split.wares + split.machines + split.dwellings;

  const setField = (key: string, value: unknown) => setPatch((p) => ({ ...p, [key]: value }));
  const clearField = (key: string) =>
    setPatch((p) => {
      const q = { ...p };
      delete q[key];
      return q;
    });

  const build = (): ProposalKind | string => {
    switch (current) {
      case "resolution":
        if (text.trim() === "") return "A resolution is its text: write it.";
        return "resolution";
      case "policy_change": {
        const p = { ...patch };
        if ("materials_split" in p) {
          if (splitSum !== 100) return `The Materials split must make 100%; it makes ${splitSum}.`;
          p.materials_split = splitToFractions(split);
        }
        if (Object.keys(p).length === 0) return "Tick at least one field to change.";
        return { policy_change: { patch: p } };
      }
      case "honor":
        if (honoree === "") return "Name the citizen to honor.";
        return { honor: { citizen: honoree } };
      case "recall":
        if (recallOffice === "" || recallWho === "") return "Name the office and the holder to recall.";
        return { recall: { office: recallOffice, citizen: recallWho } };
      default:
        return `Nothing here moves a ${kindTitle(current)}.`;
    }
  };

  const submit = () => {
    setError(null);
    setMoved(null);
    if (title.trim() === "") {
      setError("Give it a title.");
      return;
    }
    const k = build();
    if (typeof k === "string" && k !== "resolution") {
      setError(k);
      return;
    }
    propose.mutate(
      { title: title.trim(), text: text.trim(), kind: k as ProposalKind },
      {
        onSuccess: () => {
          setMoved(title.trim());
          onMoved?.(title.trim());
          setTitle("");
          setText("");
          setPatch({});
          setHonoree("");
          setRecallWho("");
        },
        onError: (e) => setError(e.message),
      },
    );
  };

  return (
    <form
      className="flex flex-col gap-4"
      data-testid="ballot-builder"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      {kinds.length > 1 ? (
        <label className="flex items-center gap-2 text-sm">
          <span>Kind</span>
          <select
            aria-label="Proposal kind"
            className="border-line rounded-sm border px-1"
            value={current}
            onChange={(e) => {
              setKind(e.target.value);
              setError(null);
            }}
          >
            {kinds.map((k) => (
              <option key={k} value={k}>
                {kindTitle(k)}
              </option>
            ))}
          </select>
        </label>
      ) : (
        <p className="text-sm">{kindTitle(current)}</p>
      )}

      <label className="flex flex-col gap-1 text-sm">
        <span>Title</span>
        <input
          aria-label="Proposal title"
          maxLength={120}
          className="border-line rounded-sm border px-2 py-1"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
      </label>

      {current === "policy_change" ? (
        <fieldset className="flex flex-col gap-3" data-testid="policy-fields">
          <legend className="text-muted text-xs">Tick the fields the change sets; the rest stay as they are.</legend>
          {fields.map((f) => {
            const on = f.key in patch;
            return (
              <div key={f.key} className="flex flex-col gap-1 text-sm">
                <label className="flex items-baseline gap-2">
                  <input
                    type="checkbox"
                    aria-label={`Change ${f.label.toLowerCase()}`}
                    checked={on}
                    onChange={(e) => {
                      if (!e.target.checked) clearField(f.key);
                      else if (f.kind === "select") setField(f.key, f.options![0]);
                      else if (f.kind === "hours") setField(f.key, 6);
                      else setField(f.key, split);
                    }}
                  />
                  <span>
                    {f.label} <span className="text-muted text-xs">{f.hint}</span>
                    {f.note ? <span className="text-warn ml-1 text-xs">({f.note})</span> : null}
                  </span>
                </label>
                {on && f.kind === "select" ? (
                  <select
                    aria-label={f.label}
                    className="border-line ml-6 w-48 rounded-sm border px-1"
                    value={String(patch[f.key])}
                    onChange={(e) => setField(f.key, e.target.value)}
                  >
                    {f.options!.map((o) => (
                      <option key={o} value={o}>
                        {fieldName(o)}
                      </option>
                    ))}
                  </select>
                ) : null}
                {on && f.kind === "hours" ? (
                  <span className="ml-6 flex items-center gap-2">
                    <input
                      type="number"
                      inputMode="numeric"
                      min={0}
                      max={24}
                      step={1}
                      aria-label={f.label}
                      className="border-line num w-20 rounded-sm border px-1"
                      value={Number(patch[f.key])}
                      onChange={(e) => setField(f.key, Math.min(24, Math.max(0, Math.trunc(Number(e.target.value) || 0))))}
                    />
                    <span className="text-muted text-xs">hours a day</span>
                  </span>
                ) : null}
                {on && f.kind === "split" ? (
                  <span className="ml-6 flex flex-wrap items-center gap-3">
                    {(["wares", "machines", "dwellings"] as const).map((sink) => (
                      <label key={sink} className="flex items-center gap-1">
                        <span className="text-muted text-xs capitalize">{sink}</span>
                        <input
                          type="number"
                          inputMode="numeric"
                          min={0}
                          max={100}
                          step={1}
                          aria-label={`${f.label}: ${sink}`}
                          className="border-line num w-16 rounded-sm border px-1"
                          value={split[sink]}
                          onChange={(e) =>
                            setSplit((s) => ({ ...s, [sink]: Math.min(100, Math.max(0, Math.trunc(Number(e.target.value) || 0))) }))
                          }
                        />
                        <span className="text-muted text-xs">%</span>
                      </label>
                    ))}
                    <span className={`num text-xs ${splitSum === 100 ? "text-muted" : "text-bad"}`}>= {splitSum}%</span>
                  </span>
                ) : null}
              </div>
            );
          })}
        </fieldset>
      ) : null}

      {current === "honor" ? (
        <label className="flex flex-col gap-1 text-sm">
          <span>Citizen to honor</span>
          <select
            aria-label="Citizen to honor"
            className="border-line w-56 rounded-sm border px-1"
            value={honoree}
            onChange={(e) => setHonoree(e.target.value === "" ? "" : Number(e.target.value))}
          >
            <option value="">choose</option>
            {citizens.map((c) => (
              <option key={c.id} value={c.id}>
                {c.handle}
              </option>
            ))}
          </select>
          <span className="text-muted text-xs">One line on their record and the scoreboard's count; never revoked.</span>
        </label>
      ) : null}

      {current === "recall" ? (
        <div className="flex flex-wrap gap-4 text-sm">
          <label className="flex flex-col gap-1">
            <span>Office</span>
            <select
              aria-label="Office to recall from"
              className="border-line w-48 rounded-sm border px-1"
              value={recallOffice}
              onChange={(e) => {
                setRecallOffice(e.target.value);
                setRecallWho("");
              }}
            >
              <option value="">choose</option>
              {heldOffices.map((o) => (
                <option key={o.kind} value={o.kind}>
                  {fieldName(o.kind)}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1">
            <span>Holder</span>
            <select
              aria-label="Holder to recall"
              className="border-line w-48 rounded-sm border px-1"
              value={recallWho}
              disabled={recallOffice === ""}
              onChange={(e) => setRecallWho(e.target.value === "" ? "" : Number(e.target.value))}
            >
              <option value="">choose</option>
              {recallHolders.map((h) => (
                <option key={h.citizen} value={h.citizen}>
                  {h.handle}
                </option>
              ))}
            </select>
          </label>
          {heldOffices.length === 0 ? <p className="text-muted self-end text-xs">No office is held; there is nobody to recall.</p> : null}
        </div>
      ) : null}

      <label className="flex flex-col gap-1 text-sm">
        <span>{current === "resolution" ? "The resolution" : "Your case"}</span>
        <textarea
          aria-label={current === "resolution" ? "Resolution text" : "Proposal text"}
          rows={3}
          maxLength={4000}
          className="border-line rounded-sm border px-2 py-1"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        {current === "resolution" ? (
          <span className="text-muted text-xs">Recorded as the assembly's minutes; it changes no rule.</span>
        ) : (
          <span className="text-muted text-xs">Optional. The floor is for the argument; this is the motion.</span>
        )}
      </label>

      <div className="flex flex-wrap items-baseline gap-3 text-sm">
        <button type="submit" disabled={propose.isPending} className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50">
          Move it
        </button>
        {moved ? <span className="text-muted">Moved. It closes at the end of the day.</span> : null}
        {error ? (
          <span className="text-bad" role="alert">
            {error}
          </span>
        ) : null}
      </div>
    </form>
  );
}
