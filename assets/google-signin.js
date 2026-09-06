(() => {
  const target = document.getElementById("google-signin-button");
  if (!target) return;
  const nonce = target.dataset.nonce;
  const start = (clientId) => {
    if (!window.google?.accounts?.id || !clientId || !nonce) return false;
    window.google.accounts.id.initialize({
      client_id: clientId, nonce,
      callback: async ({ credential }) => {
        const response = await fetch("/api/organizer/session", { method: "POST", credentials: "same-origin", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ id_token: credential, nonce }) });
        if (response.ok) window.location.reload();
        else target.setAttribute("aria-label", "Googleログインを確認できませんでした");
      },
    });
    window.google.accounts.id.renderButton(target, { theme: "outline", size: "large", width: 320 });
    return true;
  };
  fetch("/api/organizer/config", { credentials: "same-origin" }).then((r) => r.json()).then(({ client_id }) => {
    if (start(client_id)) return;
    const script = document.createElement("script");
    script.src = "https://accounts.google.com/gsi/client";
    script.async = true; script.onload = () => start(client_id);
    document.head.appendChild(script);
  });
})();
