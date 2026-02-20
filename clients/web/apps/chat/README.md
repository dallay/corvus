# Chat

Corvus conversational AI interface built with Vue 3 + Vite and shadcn-vue style components.

## Features

- ChatGPT-style conversational workspace aligned with the Corvus system design:
  - Header with model name
  - Chat panel with user/assistant bubbles
  - Gateway config panel (base URL, pairing code, bearer token, webhook secret)
  - Message composer with send action
- Local state only (mock assistant responses for now)
- Tailwind CSS v4 styling with reusable shadcn-vue-inspired UI primitives (`Button`, `Input`)

## Run

```bash
# From clients/web
pnpm install
pnpm dev:chat
```

Chat runs on <http://localhost:4323>.
