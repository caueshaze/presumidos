import type { ReactNode } from "react";

export function Section({
  id,
  title,
  children,
}: {
  id?: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-24 space-y-3">
      <h2 className="text-xl">{title}</h2>
      <div className="space-y-3 text-sm leading-6 text-ink-muted">{children}</div>
    </section>
  );
}
