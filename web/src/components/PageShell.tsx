import { motion } from "framer-motion";
import { cn } from "@/lib/utils";

/** Container central (equivalente à classe `.page`) com transição de entrada. */
export function PageShell({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <motion.main
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
      className={cn("mx-auto w-full max-w-[1100px] px-5 py-8", className)}
    >
      {children}
    </motion.main>
  );
}

/** Variante estreita para formulários de auth. */
export function FormShell({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <PageShell className="max-w-[460px]">
      <div className="rounded-lg bg-card p-8 shadow-card">
        <h1 className="mb-1 text-2xl">{title}</h1>
        {subtitle && <p className="mb-5 text-ink-muted">{subtitle}</p>}
        {children}
      </div>
    </PageShell>
  );
}
