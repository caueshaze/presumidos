import { AnimatePresence, motion } from "framer-motion";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/hooks/useTheme";
import { cn } from "@/lib/utils";

interface ThemeToggleProps {
  className?: string;
}

/** Alterna entre tema claro e escuro com micro-animação do ícone. */
export function ThemeToggle({ className }: ThemeToggleProps) {
  const { theme, toggleTheme } = useTheme();
  const isDark = theme === "dark";

  return (
    <button
      type="button"
      onClick={toggleTheme}
      aria-label={isDark ? "Ativar tema claro" : "Ativar tema escuro"}
      title={isDark ? "Tema claro" : "Tema escuro"}
      className={cn(
        "relative inline-flex h-11 w-11 items-center justify-center overflow-hidden rounded-2xl",
        "border border-mint-dark/15 bg-card/75 text-ink shadow-sm transition-colors",
        "hover:bg-card focus-visible:outline-none focus-visible:shadow-glow",
        className,
      )}
    >
      <AnimatePresence initial={false} mode="wait">
        <motion.span
          key={theme}
          initial={{ y: 12, opacity: 0, rotate: -30 }}
          animate={{ y: 0, opacity: 1, rotate: 0 }}
          exit={{ y: -12, opacity: 0, rotate: 30 }}
          transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
          className="flex items-center justify-center"
        >
          {isDark ? (
            <Moon className="h-5 w-5 text-sky" />
          ) : (
            <Sun className="h-5 w-5 text-yellow-dark" />
          )}
        </motion.span>
      </AnimatePresence>
    </button>
  );
}
