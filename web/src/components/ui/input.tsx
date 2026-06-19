import { forwardRef } from "react";
import { cn } from "@/lib/utils";

export const Input = forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "w-full rounded-md border-2 border-mint/40 bg-card px-4 py-2.5 text-ink",
        "placeholder:text-ink-muted/70 transition-colors duration-200",
        "focus:outline-none focus:border-mint-dark focus:shadow-glow",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";
