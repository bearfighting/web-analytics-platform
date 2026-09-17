FROM node:22-bookworm-slim

WORKDIR /workspace

RUN npm install --global pnpm@11.5.2

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY examples/nextjs-router-playground/package.json examples/nextjs-router-playground/package.json

RUN pnpm install --frozen-lockfile

COPY examples/nextjs-router-playground ./examples/nextjs-router-playground

ENV HOSTNAME=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["pnpm", "--filter", "@web-analytics/nextjs-router-playground", "dev", "--hostname", "0.0.0.0"]
