import { forwardRef } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-pill font-heading font-semibold text-sm transition-all duration-200 ease-smooth disabled:opacity-60 disabled:cursor-not-allowed focus-visible:outline-none focus-visible:shadow-glow active:scale-[0.97] cursor-pointer",
  {
    variants: {
      variant: {
        primary:
          "bg-mint-dark text-white shadow-card hover:bg-mint-dark/90 hover:shadow-card-hover hover:-translate-y-0.5",
        secondary:
          "bg-sky text-ink shadow-card hover:bg-sky-dark hover:shadow-card-hover hover:-translate-y-0.5",
        outline:
          "border-2 border-mint-dark/40 text-ink bg-white/60 hover:bg-white hover:border-mint-dark",
        link: "text-mint-dark underline-offset-4 hover:underline px-0",
      },
      size: {
        default: "px-5 py-2.5",
        sm: "px-3 py-1.5 text-xs",
        lg: "px-7 py-3 text-base",
      },
    },
    defaultVariants: { variant: "primary", size: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button ref={ref} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  ),
);
Button.displayName = "Button";

export { buttonVariants };
