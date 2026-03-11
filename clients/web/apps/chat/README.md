# Chat

Corvus conversational AI interface built with Vue 3 + Vite and shadcn-vue style components.

## Features

- ChatGPT-style conversational workspace aligned with the Corvus system design:
  - Header with model name
  - Chat panel with user/assistant bubbles
  - Gateway config panel (base URL, pairing code, bearer token, webhook secret)
  - Message composer with send action
- Real gateway integration using `/pair` and `/webhook`
- Tailwind CSS v4 styling with reusable shadcn-vue-inspired UI primitives (`Button`, `Input`)

## Run

```bash
# From clients/web
pnpm install
pnpm dev:chat
```

Chat runs on <http://chat.localhost:1355> via portless.

Use `PORTLESS=0 pnpm dev:chat` to run on <http://localhost:4323>.
