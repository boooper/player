# ── Stage 1: install dependencies ────────────────────────────────────────────
FROM node:22-slim AS deps
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 make g++ \
    && rm -rf /var/lib/apt/lists/*

# Copy only manifests first for better layer caching
COPY package.json package-lock.json ./
COPY packages/shared/package.json ./packages/shared/
COPY apps/api/package.json ./apps/api/

RUN npm ci --ignore-scripts

# ── Stage 2: copy source + generate Prisma client ────────────────────────────
FROM deps AS builder

COPY packages/shared ./packages/shared
COPY apps/api ./apps/api

RUN npm run db:generate --workspace @player/api

# ── Stage 3: runtime image ────────────────────────────────────────────────────
FROM node:22-slim AS runner
WORKDIR /app

# Copy everything the app needs at runtime
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/packages ./packages
COPY --from=builder /app/apps/api ./apps/api
COPY --from=builder /app/package.json ./

ENV NODE_ENV=production
ENV PORT=8787
# Override this volume-mount path via -e DATABASE_URL=file:/your/path/player.db
ENV DATABASE_URL=file:/data/player.db

EXPOSE 8787
VOLUME /data

# Run pending migrations then start the server
CMD ["sh", "-c", \
  "npm run db:deploy --workspace @player/api && \
   node_modules/.bin/tsx apps/api/src/index.ts"]
