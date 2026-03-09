# EmailBrain

Local-first AI assistant that digests your email and surfaces insights. The live application stack is now centered on Swift, Rust, and the existing web frontend; Python is only retained for the optional LoRA training utilities under `adapters/`.

## Ports

- Frontend: `http://localhost:3900`
- Backend API: `http://localhost:3901`
- LM Studio: `http://127.0.0.1:1234`

## Backend setup

```bash
cd backend
cargo run
```

## Frontend setup

```bash
cd frontend
npm install
npm run dev
```

## Environment

Copy `.env.example` to `.env` if you need custom origins or service URLs.

By default the backend uses LM Studio for chat so adapter-backed requests work on the primary product path. Set `EMAILBRAIN_INFERENCE_PROVIDER=jkca` to use `jkca-agent` as the base-model fallback for non-adapter chat.

## Notes

- The backend auto-initializes `backend/db/mail.db` on first start.
- A seed email and sample adapter are inserted into an empty database so the UI is usable before Mail.app ingestion is wired up.
- The Swift app posts extracted emails to `http://localhost:3901/api/v1/emails`.
- Adapter-backed chat is always routed through the LM Studio path until `jkca-agent` supports EmailBrain's LoRA contract.
- The remaining Python scripts in `adapters/` are optional ML tooling for LoRA training and adapter verification against the local EmailBrain API.
