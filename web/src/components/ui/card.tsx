import { forwardRef } from "react";
import { motion, type HTMLMotionProps } from "framer-motion";
import { cn } from "@/lib/utils";

export const Card = forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "rounded-lg border border-transparent bg-card p-6 shadow-card dark:border-mint/10 transition-shadow duration-200",
        className,
      )}
      {...props}
    />
  ),
);
Card.displayName = "Card";

/** Card animado para entradas em lista (stagger via index). */
export function MotionCard({ className, ...props }: HTMLMotionProps<"div">) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: [0.22, 1, 0.36, 1] }}
      className={cn("rounded-lg border border-transparent bg-card p-6 shadow-card dark:border-mint/10", className)}
      {...props}
    />
  );
}
