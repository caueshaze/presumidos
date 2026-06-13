import { motion } from "framer-motion";

export function AuthPendingCard({ message }: { message: string }) {
  return (
    <div className="mx-auto flex max-w-[1100px] justify-center px-5 py-16">
      <div className="flex flex-col items-center gap-4 rounded-lg bg-card px-10 py-12 shadow-card">
        <motion.div
          className="h-10 w-10 rounded-full border-4 border-mint/40 border-t-mint-dark"
          animate={{ rotate: 360 }}
          transition={{ repeat: Infinity, duration: 0.9, ease: "linear" }}
          aria-hidden
        />
        <p className="text-ink-muted">{message}</p>
      </div>
    </div>
  );
}
