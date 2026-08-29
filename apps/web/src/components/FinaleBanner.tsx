import { motion } from "framer-motion";
import { Sparkles } from "lucide-react";

/** Destaque de um bolão configurado pelo admin, sem assumir esporte ou evento. */
export function FinaleBanner({ poolName, eventName }: { poolName: string; eventName: string }) {
  return (
    <motion.section
      initial={{ opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
      className="finale-banner"
      aria-label={`Bolão em destaque: ${poolName}`}
    >
      <div className="finale-banner__lights" aria-hidden="true" />
      <div className="finale-banner__content">
        <div className="finale-banner__eyebrow">
          <Sparkles className="h-4 w-4" aria-hidden="true" />
          Em destaque
        </div>
        <div className="finale-banner__match">
          <span className="finale-banner__team">{poolName}</span>
        </div>
        <p className="finale-banner__caption">{eventName}</p>
      </div>
    </motion.section>
  );
}
