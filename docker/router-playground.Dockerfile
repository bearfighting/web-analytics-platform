FROM node:22-bookworm-slim

WORKDIR /workspace

RUN npm install --global pnpm@11.5.2

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps ./apps
COPY examples ./examples
COPY packages ./packages

RUN find packages -name dist -prune -exec rm -rf {} + && find packages -name '*.tsbuildinfo' -delete && chown -R node:node /workspace
USER node
RUN pnpm install --frozen-lockfile
RUN pnpm build:packages

ENV HOSTNAME=0.0.0.0
ENV PORT=3000
ENV CI=true

EXPOSE 3000 3101 3102
