# Demo sandbox

- URL: `https://webhook-quiet-hours.sociobot.in/demo`
- Entry: the landing page’s **Try it with sample data** link opens it in one click.
- Sample: one signed “Deploy monitor” alias and 18 deliveries grouped into three realistic fingerprints: a high deployment failure, a normal invoice-sync failure, and a record-only backup completion.
- Isolation: `POST /api/demo/session` provisions a random 32-character workspace and returns the complete sample. The browser carries changes only in `demo:webhook-quiet-hours:state` session storage, separate from all real keys. This keeps the demo consistent across container replicas. Demo code never queries or mutates the production SQLite tenant.
- Lifetime: the provision response expires after 24 hours. The browser rejects an expired state and provisions a new one. **Start for real** clears both demo keys, asks the server to discard its ephemeral copy, and returns to the admin-token screen.
- Reset: **Reset demo** provisions the original sample under a new random workspace and replaces only the demo namespace.
- Safety: demo notification destinations are disabled. Sending a sample digest updates only in-memory counts and never makes an outbound notification request.
