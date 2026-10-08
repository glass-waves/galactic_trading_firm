#!/usr/bin/env bash
# install (or refresh) the user-level systemd units for unattended paper trading.
# idempotent. run from anywhere. requires: podman socket, docker-compose shim,
# ~/.cargo/bin/sqlx, a release build (cargo build --release -p data_feed).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DEST="$HOME/.config/systemd/user"
mkdir -p "$DEST"
for f in "$HERE"/*.service "$HERE"/*.timer; do
    install -m 0644 "$f" "$DEST/$(basename "$f")"
done
systemctl --user daemon-reload
systemctl --user enable --now podman.socket
systemctl --user enable trading-postgres.service
systemctl --user enable --now paper-trader.timer paper-trader-stop.timer preopen-check.timer eod-review.timer watchdog.timer export-day.timer research-runner.timer
# research pipeline runner (20:30 PT weekdays; scripts/pipeline/pipeline.py advance + report). the unit files ship with the pipeline.
if [[ -f "$HERE/pipeline.timer" ]]; then systemctl --user enable --now pipeline.timer; else echo "note: pipeline.timer not present in $HERE — skipped"; fi
# stage-0 edge pretest (21:15 PT weekdays, after the pipeline job; research/pretest/harness.py via scripts/pretest_nightly.sh). the unit files ship with the pretest harness.
if [[ -f "$HERE/pretest.timer" ]]; then systemctl --user enable --now pretest.timer; else echo "note: pretest.timer not present in $HERE — skipped"; fi
# weekly ideation check-in (Sunday 18:00 PT; proposes new research/pretest/cells.py cells). the unit files ship with the ideate skill.
if [[ -f "$HERE/ideate.timer" ]]; then systemctl --user enable --now ideate.timer; else echo "note: ideate.timer not present in $HERE — skipped"; fi
# the intraday LLM review is on-demand since 2026-09-14 (run it from a session with /intraday-review or /loop 15m /intraday-review)
systemctl --user disable --now intraday-review.timer 2>/dev/null || true
# keep user services alive without an interactive login (survives logout / reboot)
loginctl enable-linger "$USER" || echo "warning: could not enable linger (run: sudo loginctl enable-linger $USER)"
echo
systemctl --user list-timers --all --no-pager | grep -E "paper-trader|preopen|intraday|eod|watchdog|export|research|pipeline|pretest|ideate|NEXT" || true
echo
echo "units installed. manual controls:"
echo "  systemctl --user start|stop|status paper-trader.service"
echo "  journalctl --user -u paper-trader.service -f"
echo "  systemctl --user start intraday-review.service   # run a check-in now; output in logs/checkins/"
