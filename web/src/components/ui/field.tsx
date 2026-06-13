import { forwardRef } from "react";
import { cn } from "@/lib/utils";

export function Label({ className, ...props }: React.LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label
      className={cn("mb-1.5 block text-sm font-semibold text-ink-muted", className)}
      {...props}
    />
  );
}

export const Select = forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "w-full rounded-md border-2 border-mint/40 bg-white px-4 py-2.5 text-ink",
        "focus:outline-none focus:border-mint-dark focus:shadow-glow",
        className,
      )}
      {...props}
    />
  ),
);
Select.displayName = "Select";

export function ErrorBanner({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-md border border-danger/50 bg-danger-bg px-4 py-3 text-sm font-semibold text-ink">
      {children}
    </div>
  );
}
