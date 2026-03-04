#!/bin/bash
# Porthouse: pre-server port conflict check for Claude Code
# Only activates when a dev server command is detected.
# Silent when no conflict or when command is not a server start.

# Exit immediately if porthouse is not installed
command -v porthouse &>/dev/null || exit 0

# Only check for server-start commands
echo "$@" | grep -qE '(npm run dev|npm start|yarn dev|yarn start|flask run|uvicorn|cargo run|python.*manage\.py runserver|next dev|vite|ng serve|rails s)' || exit 0

# Run conflict check — silent if no conflicts
porthouse check --quiet 2>/dev/null
if [ $? -ne 0 ]; then
    echo "[porthouse] Port conflict detected. Free ports: $(porthouse suggest 3 2>/dev/null | tr '\n' ' ')"
fi
