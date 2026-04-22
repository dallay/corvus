const root = document.getElementById("app");

if (root) {
  root.innerHTML = `
    <main style="min-height: 100vh; margin: 0; background: radial-gradient(circle at top, rgba(39,72,116,0.28), transparent 40%), linear-gradient(180deg, #09111d 0%, #07111f 100%); color: #e6eef8; font-family: Inter, system-ui, sans-serif;">
      <section style="max-width: 52rem; margin: 0 auto; padding: 4rem 1.5rem 3rem;">
        <p style="margin: 0 0 0.75rem; font-size: 0.875rem; letter-spacing: 0.08em; text-transform: uppercase; color: #7dd3fc;">Corvus Rook</p>
        <h1 style="margin: 0 0 1rem; font-size: 2.5rem;">Dedicated operator dashboard surface</h1>
        <p style="margin: 0 0 1.5rem; max-width: 40rem; color: #bfd2e8; line-height: 1.6;">This embedded entrypoint now belongs to the Rook-specific dashboard app. Build <code>clients/web/apps/rook-dashboard</code> to replace this fallback bundle with the real overview and provider/account workflows for OpenSpec change #592.</p>
        <nav aria-label="Rook dashboard sections" style="display: flex; gap: 0.75rem; flex-wrap: wrap; margin-bottom: 1.5rem;">
          <a href="#/overview" style="padding: 0.75rem 1rem; border-radius: 999px; background: rgba(125, 211, 252, 0.14); color: #d8f3ff; text-decoration: none;">Overview</a>
          <a href="#/accounts" style="padding: 0.75rem 1rem; border-radius: 999px; background: rgba(125, 211, 252, 0.14); color: #d8f3ff; text-decoration: none;">Providers & accounts</a>
        </nav>
        <div style="padding: 1rem 1.25rem; border-radius: 1rem; border: 1px solid rgba(125, 211, 252, 0.18); background: rgba(8, 15, 26, 0.72); color: #bfd2e8; line-height: 1.5;">
          <strong style="display: block; margin-bottom: 0.35rem; color: #e6eef8;">Scope guardrail</strong>
          <span>#592 is limited to the shell, overview, and provider/account flows. Pools, routes, usage, logs, settings, and backups stay deferred to #593/#594.</span>
        </div>
      </section>
    </main>
  `;
}
