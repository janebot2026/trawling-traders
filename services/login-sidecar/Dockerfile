FROM node:20-slim

WORKDIR /app

COPY package.json package-lock.json* ./
RUN npm ci --omit=dev

# Bind to all interfaces inside the container
ENV HOST=0.0.0.0
ENV PORT=3100

EXPOSE 3100

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -sf http://localhost:3100/health || exit 1

# Install curl for healthcheck
RUN apt-get update -qq && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*

CMD ["npm", "start"]
