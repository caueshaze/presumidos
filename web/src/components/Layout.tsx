import { Link, Outlet } from "react-router-dom";
import { Navbar } from "./Navbar";

export function Layout() {
  return (
    <div className="flex min-h-screen flex-col">
      <Navbar />
      <div className="flex-1">
        <Outlet />
      </div>
      <footer className="mx-auto w-full max-w-[1100px] px-5 pb-8 pt-4 text-sm text-ink-muted">
        <div className="flex flex-wrap items-center justify-center gap-x-4 gap-y-2 rounded-lg border border-mint-dark/10 bg-white/55 px-4 py-4 text-center shadow-sm backdrop-blur-sm">
          <Link to="/terms" className="font-semibold text-mint-dark hover:underline">
            Termos de Uso
          </Link>
          <span className="hidden text-mint-dark/40 sm:inline">|</span>
          <Link to="/privacy" className="font-semibold text-mint-dark hover:underline">
            Política de Privacidade
          </Link>
          <span className="hidden text-mint-dark/40 sm:inline">|</span>
          <Link to="/contact" className="font-semibold text-mint-dark hover:underline">
            Contato
          </Link>
        </div>
      </footer>
    </div>
  );
}
