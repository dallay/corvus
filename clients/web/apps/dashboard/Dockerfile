# syntax=docker/dockerfile:1

FROM node:22-alpine AS build

WORKDIR /app

RUN corepack enable

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/dashboard/package.json ./apps/dashboard/package.json
COPY packages/locales/package.json ./packages/locales/package.json
COPY packages/shared/package.json ./packages/shared/package.json
COPY packages/ui/package.json ./packages/ui/package.json

RUN pnpm install --frozen-lockfile --node-linker=hoisted --ignore-scripts

COPY apps/dashboard ./apps/dashboard
COPY packages/locales ./packages/locales
COPY packages/shared ./packages/shared
COPY packages/ui ./packages/ui

WORKDIR /app/apps/dashboard

RUN pnpm exec vite build

FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime

COPY --from=build /app/apps/dashboard/dist /usr/share/nginx/html

EXPOSE 8080
