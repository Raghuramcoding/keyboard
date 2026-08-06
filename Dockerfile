# T.C.K (TalentCloud Keyboard) — production image.
# Stage 1 installs dependencies and builds the static UI (Vite); stage 2 is a
# slim runtime that serves the bundle and runs the terminal/AI/GitHub backend.

# ---------- build ----------
FROM node:22-bookworm-slim AS build
WORKDIR /app

# node-pty is a native addon, so the build stage needs a toolchain.
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 make g++ \
 && rm -rf /var/lib/apt/lists/*

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

# ---------- runtime ----------
FROM node:22-bookworm-slim
# ca-certificates: outbound HTTPS to GitHub/Anthropic/providers. bash: the PTY shell.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates bash git curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -u 10001 tck

WORKDIR /app
COPY --from=build /app/node_modules /app/node_modules
COPY --from=build /app/package.json /app/package.json
COPY --from=build /app/dist /app/dist
COPY --from=build /app/src /app/src

ENV TCK_DIST=/app/dist \
    PORT=3000

# Run as an unprivileged user — the terminal grants a shell, so limit its reach.
USER tck
EXPOSE 3000
CMD ["npm", "run", "start"]
