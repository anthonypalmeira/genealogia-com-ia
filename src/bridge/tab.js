(() => {
  const bridge = "__BRIDGE__";
  const site = "__SITE__";
  if (location.origin !== site) {
    return "aba errada: abra " + site + " e avalie o trecho de novo";
  }
  if (window.garimpoBridge === bridge) {
    return "ligada";
  }
  window.garimpoBridge = bridge;
  const session = (job) => {
    const cookie = document.cookie.split("; ").find((c) => c.startsWith("fssessionid="));
    const value = cookie ? cookie.slice("fssessionid=".length) : null;
    return { id: job.id, answer: { status: value ? 200 : 401, retryAfter: null, processingTime: null, location: null, body: value } };
  };
  const run = async (job) => {
    const url = new URL(job.path, location.origin);
    if (url.origin !== location.origin) {
      return { id: job.id, error: "path outside the site" };
    }
    const headers = { Accept: "application/json" };
    const cookie = document.cookie.split("; ").find((c) => c.startsWith("fssessionid="));
    if (cookie) {
      headers.Authorization = "Bearer " + cookie.slice("fssessionid=".length);
    }
    try {
      const response = await fetch(url, { headers, redirect: "manual" });
      return {
        id: job.id,
        answer: {
          status: response.type === "opaqueredirect" ? 302 : response.status,
          retryAfter: response.headers.get("retry-after"),
          processingTime: response.headers.get("x-processing-time"),
          location: response.headers.get("location"),
          body: response.ok ? await response.text().catch(() => null) : null,
        },
      };
    } catch (error) {
      return { id: job.id, error: String(error) };
    }
  };
  (async () => {
    while (window.garimpoBridge === bridge) {
      try {
        const next = await fetch(bridge + "/proxima");
        if (next.status === 204) {
          continue;
        }
        if (!next.ok) {
          break;
        }
        const job = await next.json();
        const accepted = await fetch(bridge + "/aceito", { method: "POST", body: String(job.id) });
        if (!accepted.ok) {
          continue;
        }
        const done = job.session ? session(job) : await run(job);
        await fetch(bridge + "/resposta", { method: "POST", body: JSON.stringify(done) });
      } catch (error) {
        break;
      }
    }
    if (window.garimpoBridge === bridge) {
      delete window.garimpoBridge;
    }
  })();
  return "ligada";
})()
