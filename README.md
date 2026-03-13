# Player Monorepo

## Structure

- `apps/web`: SvelteKit frontend (adapter-static)
- `apps/desktop`: Tauri shell that wraps `apps/web`
- `apps/api`: Hono backend service
- `packages/shared`: shared contracts/constants/types for web/api

## Commands

- `npm run dev:web`
- `npm run dev:desktop`
- `npm run dev:api`
- `npm run build:web`
- `npm run build:desktop`
- `npm run build:api`
