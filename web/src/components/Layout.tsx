import { Link, Outlet } from "react-router-dom";
import { usePublicSettings } from "@/hooks/queries";
import { Navbar } from "./Navbar";

export function Layout() {
  const settings = usePublicSettings();
  const showBanner =
    settings.data?.globalBannerEnabled && settings.data.globalBannerText.trim().length > 0;

  return (
    <div className="flex min-h-screen flex-col">
      <Navbar />
      {showBanner && (
        <div className="border-b border-yellow/40 bg-yellow/25 px-5 py-3 text-sm text-ink">
          <div className="mx-auto max-w-[1100px] font-medium">{settings.data?.globalBannerText}</div>
        </div>
      )}
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
