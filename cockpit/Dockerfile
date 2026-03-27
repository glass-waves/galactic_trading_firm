FROM node:22-slim AS base

WORKDIR /app

# install dependencies
COPY cockpit/package.json cockpit/package-lock.json* ./
RUN npm ci --production=false

# copy source
COPY cockpit/ .

# build
RUN npm run build

# production
FROM node:22-slim AS runner
WORKDIR /app
ENV NODE_ENV=production

COPY --from=base /app/.next/standalone ./
COPY --from=base /app/.next/static ./.next/static
COPY --from=base /app/public ./public

EXPOSE 3000
CMD ["node", "server.js"]
