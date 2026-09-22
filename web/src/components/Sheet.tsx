// Sheet (docs/style.md §7.20): anything that opens over the page on a phone —
// the More menu, a long note, an order form — is a bottom sheet with a drag
// handle; from md it is a centred dialog. A native <dialog>, so Escape and
// the backdrop close it and focus stays inside.

import { useEffect, useRef, type ReactNode } from "react";

export function Sheet({ open, onClose, title, children, testId }: { open: boolean; onClose: () => void; title: ReactNode; children: ReactNode; testId?: string }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal?.();
    if (!open && d.open) d.close();
  }, [open]);
  return (
    <dialog
      ref={ref}
      data-testid={testId}
      className="sheet"
      aria-labelledby={`${testId ?? "sheet"}-title`}
      onClose={onClose}
      onClick={(e) => {
        // A click on the backdrop (the dialog element itself, outside its box) closes.
        if (e.target === ref.current) onClose();
      }}
    >
      <div className="bg-line-strong mx-auto mb-3 h-1 w-9 rounded-[2px] md:hidden" aria-hidden />
      <h2 id={`${testId ?? "sheet"}-title`} className="mb-2 text-[17px]">
        {title}
      </h2>
      {children}
    </dialog>
  );
}

/** A row inside a sheet's list: icon + label, 44px tall, one line beneath. */
export function SheetRow({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={["border-line flex min-h-touch items-center gap-2.5 border-b px-1 py-2.5 last:border-b-0", className ?? ""].join(" ")}>{children}</div>;
}
