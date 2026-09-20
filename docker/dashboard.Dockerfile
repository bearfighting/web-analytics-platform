FROM node:22-bookworm-slim

WORKDIR /workspace

RUN corepack enable \
  && corepack install --global pnpm@11.5.2 \
  && apt-get update \
  && apt-get install --no-install-recommends --yes curl \
  && rm -rf /var/lib/apt/lists/*

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/dashboard/package.json apps/dashboard/package.json
COPY examples/nextjs-router-playground/package.json examples/nextjs-router-playground/package.json
COPY packages/analytics-browser/package.json packages/analytics-browser/package.json
COPY packages/analytics-core/package.json packages/analytics-core/package.json
COPY packages/observer-core/package.json packages/observer-core/package.json
COPY packages/observer-next/package.json packages/observer-next/package.json
COPY packages/protocol-ts/package.json packages/protocol-ts/package.json
COPY packages/transport/package.json packages/transport/package.json

RUN pnpm install --frozen-lockfile

EXPOSE 3000

CMD ["pnpm", "--filter", "@web-analytics/dashboard", "dev", "--hostname", "0.0.0.0"]
