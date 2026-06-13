import { useState } from "react";
import { motion, useMotionValueEvent, useScroll } from "framer-motion";
import { NavLink, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "./ui/button";
import { cn } from "@/lib/utils";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  cn(
    "rounded-pill px-3 py-1.5 text-sm font-semibold text-ink-muted transition-colors hover:text-ink",
    isActive && "bg-mint/30 text-ink",
  );

export function Navbar() {
  const { user, loading, logout } = useAuth();
  const navigate = useNavigate();

  // Esconde ao rolar para baixo, reaparece ao rolar para cima (e sempre no topo).
  const { scrollY } = useScroll();
  const [hidden, setHidden] = useState(false);
  useMotionValueEvent(scrollY, "change", (latest) => {
    const previous = scrollY.getPrevious() ?? 0;
    if (latest > previous && latest > 80) setHidden(true);
    else setHidden(false);
  });

  const handleLogout = async () => {
    await logout();
    navigate("/");
  };

  return (
    <motion.nav
      variants={{ visible: { y: 0 }, hidden: { y: "-110%" } }}
      animate={hidden ? "hidden" : "visible"}
      transition={{ duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
      className="sticky top-0 z-30 mx-auto flex w-full max-w-[1100px] items-center gap-2 rounded-b-lg bg-bg/75 px-5 py-4 backdrop-blur-md"
    >
      <NavLink to="/" className="mr-2 font-heading text-xl font-bold text-mint-dark">
        Presumidos
      </NavLink>

      {user ? (
        <>
          <NavLink to="/dashboard" className={linkClass}>
            Dashboard
          </NavLink>
          <NavLink to="/predictions" className={linkClass}>
            Previsões
          </NavLink>
          <NavLink to="/leaderboard" className={linkClass}>
            Ranking
          </NavLink>
          <div className="flex-1" />
          <span className="hidden text-sm text-ink-muted sm:inline">Olá, {user.username}</span>
          <Button variant="outline" size="sm" onClick={handleLogout}>
            Sair
          </Button>
        </>
      ) : (
        <>
          <div className="flex-1" />
          {!loading && (
            <>
              <Button variant="secondary" size="sm" onClick={() => navigate("/login")}>
                Login
              </Button>
              <Button variant="primary" size="sm" onClick={() => navigate("/register")}>
                Criar conta
              </Button>
            </>
          )}
        </>
      )}
    </motion.nav>
  );
}
