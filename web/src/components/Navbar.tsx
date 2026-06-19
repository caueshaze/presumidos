import { useEffect, useState } from "react";
import { AnimatePresence, motion, useMotionValueEvent, useScroll } from "framer-motion";
import { Menu, X } from "lucide-react";
import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "./ui/button";
import { ThemeToggle } from "./ui/ThemeToggle";
import { cn } from "@/lib/utils";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  cn(
    "whitespace-nowrap rounded-pill px-3 py-1.5 text-sm font-semibold text-ink-muted transition-colors hover:text-ink",
    isActive && "bg-mint/30 text-ink",
  );

const mobileLinkClass = ({ isActive }: { isActive: boolean }) =>
  cn(
    "rounded-2xl px-4 py-3 text-base font-semibold text-ink-muted transition-colors hover:text-ink",
    isActive && "bg-mint/30 text-ink",
  );

export function Navbar() {
  const { user, isAdmin, loading, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const homeTarget = user && isAdmin ? "/admin" : "/";

  // Esconde ao rolar para baixo, reaparece ao rolar para cima (e sempre no topo).
  const { scrollY } = useScroll();
  const [hidden, setHidden] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  useMotionValueEvent(scrollY, "change", (latest) => {
    const previous = scrollY.getPrevious() ?? 0;
    if (latest > previous && latest > 80) setHidden(true);
    else setHidden(false);
  });

  useEffect(() => {
    setMobileOpen(false);
  }, [location.pathname]);

  const handleLogout = async () => {
    await logout();
    setMobileOpen(false);
    navigate("/");
  };

  return (
    <motion.nav
      variants={{ visible: { y: 0 }, hidden: { y: "-110%" } }}
      animate={hidden ? "hidden" : "visible"}
      transition={{ duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
      className="sticky top-0 z-30 mx-auto w-full max-w-[1100px] rounded-b-[28px] bg-bg/80 px-4 py-3 shadow-[0_14px_36px_rgba(63,77,68,0.08)] backdrop-blur-md sm:flex sm:items-center sm:px-5 sm:py-4"
    >
      <div className="flex w-full items-center justify-between gap-3">
        <NavLink
          to={homeTarget}
          className="min-w-0 font-heading text-lg font-bold text-mint-dark sm:mr-2 sm:text-xl"
        >
          <span className="block truncate">Presumidos</span>
        </NavLink>

        <div className="flex items-center gap-2 sm:hidden">
          <ThemeToggle />
          <button
            type="button"
            aria-label={mobileOpen ? "Fechar menu" : "Abrir menu"}
            aria-expanded={mobileOpen}
            onClick={() => setMobileOpen((open) => !open)}
            className="inline-flex h-11 w-11 items-center justify-center rounded-2xl border border-mint-dark/15 bg-card/75 text-ink shadow-sm transition-colors hover:bg-card"
          >
            {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </button>
        </div>

        <div className="hidden min-w-0 items-center gap-2 sm:flex sm:flex-1">
          {user ? (
            <>
              {isAdmin ? (
                <NavLink to="/admin" className={linkClass}>
                  Admin
                </NavLink>
              ) : (
                <>
                  <NavLink to="/dashboard" className={linkClass}>
                    Meus Bolões
                  </NavLink>
                  <NavLink to="/predictions" className={linkClass}>
                    Meus Palpites
                  </NavLink>
                  <NavLink to="/palpites-do-bolao" className={linkClass}>
                    Palpites do Bolão
                  </NavLink>
                  <NavLink to="/leaderboard" className={linkClass}>
                    Ranking
                  </NavLink>
                </>
              )}
              <div className="flex-1" />
              <ThemeToggle className="h-9 w-9" />
              <NavLink
                to="/conta"
                className="truncate rounded-pill px-2 py-1 text-sm text-ink-muted transition-colors hover:text-ink"
              >
                Conta
              </NavLink>
              <Button variant="outline" size="sm" onClick={handleLogout}>
                Sair
              </Button>
            </>
          ) : (
            <>
              <div className="flex-1" />
              <ThemeToggle className="h-9 w-9" />
              {!loading && (
                <div className="flex items-center gap-2">
                  <Button variant="secondary" size="sm" onClick={() => navigate("/login")}>
                    Login
                  </Button>
                  <Button variant="primary" size="sm" onClick={() => navigate("/register")}>
                    Criar conta
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      <AnimatePresence initial={false}>
        {mobileOpen && (
          <motion.div
            initial={{ opacity: 0, height: 0, y: -10 }}
            animate={{ opacity: 1, height: "auto", y: 0 }}
            exit={{ opacity: 0, height: 0, y: -10 }}
            transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
            className="overflow-hidden sm:hidden"
          >
            <div className="mt-3 flex flex-col gap-3 rounded-[24px] border border-mint-dark/10 bg-card/80 p-3 shadow-[0_12px_28px_rgba(63,77,68,0.08)]">
              {user ? (
                <>
                  <NavLink to="/conta" className={mobileLinkClass}>
                    Conta
                  </NavLink>
                  <div className="grid grid-cols-1 gap-1">
                    {isAdmin ? (
                      <NavLink to="/admin" className={mobileLinkClass}>
                        Admin
                      </NavLink>
                    ) : (
                      <>
                        <NavLink to="/dashboard" className={mobileLinkClass}>
                          Meus Bolões
                        </NavLink>
                        <NavLink to="/predictions" className={mobileLinkClass}>
                          Meus Palpites
                        </NavLink>
                        <NavLink to="/palpites-do-bolao" className={mobileLinkClass}>
                          Palpites do Bolão
                        </NavLink>
                        <NavLink to="/leaderboard" className={mobileLinkClass}>
                          Ranking
                        </NavLink>
                      </>
                    )}
                  </div>
                  <Button variant="outline" onClick={handleLogout} className="w-full justify-center">
                    Sair
                  </Button>
                </>
              ) : (
                <>
                  {!loading && (
                    <div className="flex flex-col gap-2">
                      <Button
                        variant="primary"
                        onClick={() => navigate("/register")}
                        className="w-full justify-center"
                      >
                        Criar conta
                      </Button>
                      <Button
                        variant="secondary"
                        onClick={() => navigate("/login")}
                        className="w-full justify-center"
                      >
                        Login
                      </Button>
                    </div>
                  )}
                </>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.nav>
  );
}
