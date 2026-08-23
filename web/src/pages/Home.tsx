import { Navigate, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

const benefits = [
  ["⚔️", "Disputa entre amigos", "Crie um bolão e convide quem vai tornar a resenha mais divertida."],
  ["🎯", "Palpite rápido", "Escolha, estime ou responda com uma experiência feita para o evento."],
  ["🏆", "Ranking sempre à vista", "Acompanhe resultados e descubra quem entende mais — ou teve mais sorte."],
] as const;

export function HomePage() {
  const { user } = useAuth();
  return user ? <Navigate to="/dashboard" replace /> : <MarketingHome />;
}

function MarketingHome() {
  const navigate = useNavigate();
  return <PageShell>
    <section className="mb-10 text-center"><span className="font-heading text-sm font-semibold uppercase tracking-widest text-mint-dark">Presumidos</span><h1 className="mt-2 text-4xl sm:text-5xl">Seus eventos. Seus palpites. Seus bolões.</h1><p className="mx-auto mt-3 max-w-2xl text-lg text-ink-muted">Crie bolões, faça seus palpites e dispute com seus amigos. Ainda dá tempo de virar o jogo.</p><div className="mt-6 flex flex-wrap justify-center gap-3"><Button onClick={() => navigate("/register")}>Criar conta</Button><Button variant="secondary" onClick={() => navigate("/login")}>Entrar</Button></div></section>
    <Card className="mx-auto mb-8 max-w-2xl"><h2 className="text-2xl">Escolha o evento. Chame a galera. Faça seus palpites.</h2><p className="mt-3 text-ink-muted">Perguntas, números e múltiplas escolhas convivem no mesmo lugar — cada evento com as suas próprias regras.</p></Card>
    <div className="grid gap-5 sm:grid-cols-3">{benefits.map(([icon, title, text], index) => <motion.div key={title} initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: index * 0.08, duration: 0.32 }}><Card className="h-full"><span className="text-3xl">{icon}</span><h3 className="mt-3 text-lg">{title}</h3><p className="mt-1 text-sm text-ink-muted">{text}</p></Card></motion.div>)}</div>
  </PageShell>;
}
