# Demo sandbox

- URL: `https://webhook-quiet-hours.sociobot.in/demo`
- Entry: the landing page’s **Try it with sample data** link opens it in one click.
- Sample: one signed “Deploy monitor” alias and 18 deliveries grouped into three realistic fingerprints: a high deployment failure, a normal invoice-sync failure, and a record-only backup completion.
- Isolation: `POST /api/demo/session` creates a random 32-character in-memory workspace. Demo routes can access only that workspace and never query or mutate the production SQLite tenant. The browser keeps its workspace ID under the session-storage key `demo:webhook-quiet-hours:workspace`.
- Lifetime: a workspace expires after 24 hours and is removed on the next demo access. **Start for real** deletes it immediately and returns to the admin-token screen.
- Reset: **Reset demo** replaces only the current workspace with the original sample.
- Safety: demo notification destinations are disabled. Sending a sample digest updates only in-memory counts and never makes an outbound notification request.
