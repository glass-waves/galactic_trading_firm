#!/bin/bash
set -e

# run paper_trader inside a tmux session so users can attach over SSH:
#   docker exec -it <container> tmux attach -t trader
#
# - remain-on-exit keeps the session visible after the process exits
# - exec ensures SIGTERM from docker propagates to tmux → paper_trader

exec tmux new-session -d -s trader \
    "paper_trader $*; echo 'paper_trader exited'; sleep infinity" \; \
    set-option remain-on-exit on \; \
    attach-session -t trader
