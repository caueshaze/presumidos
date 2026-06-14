self.addEventListener("push", (event) => {
  if (!event.data) return;

  let payload = null;
  try {
    payload = event.data.json();
  } catch (_error) {
    return;
  }

  const notification = {
    title: payload?.title ?? "Presumidos",
    body: payload?.body ?? "Voce tem um lembrete pendente.",
    icon: "/android-chrome-192x192.png",
    badge: "/favicon-32x32.png",
    tag: payload?.tag ?? "presumidos-reminder",
    data: {
      url: payload?.url ?? "/predictions",
      matches: payload?.matches ?? [],
    },
  };

  event.waitUntil(self.registration.showNotification(notification.title, notification));
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const targetUrl = event.notification?.data?.url ?? "/predictions";

  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
      for (const client of clients) {
        const clientUrl = new URL(client.url);
        if (clientUrl.origin === self.location.origin) {
          return client.focus().then(() => {
            if ("navigate" in client) {
              return client.navigate(targetUrl);
            }
            return client;
          });
        }
      }

      if (self.clients.openWindow) {
        return self.clients.openWindow(targetUrl);
      }

      return null;
    }),
  );
});
