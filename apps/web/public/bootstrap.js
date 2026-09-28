(function () {
  try {
    var stored = localStorage.getItem("theme");
    var dark = stored === "dark" ||
      (!stored && window.matchMedia("(prefers-color-scheme: dark)").matches);
    if (dark) {
      document.documentElement.classList.add("dark");
      var meta = document.querySelector('meta[name="theme-color"]');
      if (meta) meta.setAttribute("content", "#0e1614");
    }
  } catch (_error) {}

  var reloading = false;
  window.addEventListener("vite:preloadError", function (event) {
    if (reloading) return;
    try {
      var lastReload = Number(sessionStorage.getItem("chunk-reload-at") || "0");
      if (Date.now() - lastReload < 10000) return;
      sessionStorage.setItem("chunk-reload-at", String(Date.now()));
    } catch (_error) {}
    reloading = true;
    event.preventDefault();
    window.location.reload();
  });
})();
