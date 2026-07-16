import { motion } from "framer-motion";
import { Crown, Trophy } from "lucide-react";

/** Faixa global da edição especial, exibida apenas enquanto o admin a mantém ativa. */
export function FinaleBanner() {
  return (
    <motion.section
      initial={{ opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
      className="finale-banner"
      aria-label="A Grande Final: Espanha contra Argentina"
    >
      <div className="finale-banner__lights" aria-hidden="true" />
      <div className="finale-banner__content">
        <div className="finale-banner__eyebrow">
          <Crown className="h-4 w-4" aria-hidden="true" />
          A grande final
          <Trophy className="h-4 w-4" aria-hidden="true" />
        </div>
        <div className="finale-banner__match">
          <span className="finale-banner__team">
            <span className="finale-banner__flag" aria-hidden="true">🇪🇸</span>
            Espanha
          </span>
          <span className="finale-banner__versus">×</span>
          <span className="finale-banner__team">
            Argentina
            <span className="finale-banner__flag" aria-hidden="true">🇦🇷</span>
          </span>
        </div>
        <p className="finale-banner__caption">Faça seu palpite para a decisão.</p>
      </div>
    </motion.section>
  );
}
