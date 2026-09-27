#!/usr/bin/env bash
# replay one enabled book for a date on the IEX bar cache, into data/live/<date>/replay_<book>.csv
# usage: scripts/replay_book.sh <book> <YYYY-MM-DD>
#
# resolves the book's config (books.config_version_id, else the latest promoted row — what the
# trader runs for a follow-promoted book) and its tickers (books.tickers, else the config's own)
# from the database, then runs the backtest with the live-vs-replay cost model and NO sizing
# override (the blob's fraction, exactly as live and shadow size). export_day.sh calls this for
# every enabled book; the pipeline's parity step reads the files and calls it for missing days.
# exit: 0 ok · 2 usage / unknown book · 3 book disabled (REPLAY_BOOK_FORCE=1 overrides) · 4 backtest failed
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
set -a; [[ -f .env ]] && source .env; set +a
BOOK="${1:-}"; D="${2:-}"
if [[ -z "$BOOK" || ! "$D" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    echo "usage: scripts/replay_book.sh <book> <YYYY-MM-DD>" >&2; exit 2
fi
Q() { ./scripts/psql.sh -At -F '|' -c "$1" </dev/null; }
B_SQL="${BOOK//\'/\'\'}"
ROW="$(Q "SELECT COALESCE(config_version_id::text,''), COALESCE(array_to_string(tickers,','),''), enabled, COALESCE(capital::text,'') FROM books WHERE name='$B_SQL'")"
[[ -n "$ROW" ]] || { echo "replay_book: unknown book '$BOOK'" >&2; exit 2; }
IFS='|' read -r CFG TICKERS ENABLED CAP <<< "$ROW"
if [[ "$ENABLED" != "t" && "${REPLAY_BOOK_FORCE:-0}" != "1" ]]; then
    echo "replay_book: book '$BOOK' is disabled (REPLAY_BOOK_FORCE=1 to replay anyway)" >&2; exit 3
fi
CAPITAL="${CAP:-${INITIAL_CAPITAL:-10000}}"
PROMOTED="$(Q "SELECT max(id) FROM config_versions WHERE status='promoted'")"

ARGS=(--date "$D" --lookback-days 8 --capital "$CAPITAL" --slippage-bps 3.0 --half-spread 0.005
      --output-trades-csv --bars-dir data/bars_iex --cross-index SPY)
[[ -n "$CFG" ]] && ARGS+=(--config-id "$CFG")
[[ -n "$TICKERS" ]] && ARGS+=(--tickers "$TICKERS")

OUT="${LIVE_DIR:-data/live}/$D"; mkdir -p "$OUT"; F="$OUT/replay_$BOOK.csv"   # LIVE_DIR: test override
echo "replay_book: $BOOK $D config=${CFG:-promoted:$PROMOTED} tickers=${TICKERS:-config} capital=$CAPITAL -> $F" >&2
echo "replay_book: ./target/release/backtest ${ARGS[*]}" >&2
if ./target/release/backtest "${ARGS[@]}" > "$F.tmp" 2> "$F.err"; then
    mv "$F.tmp" "$F"; rm -f "$F.err"
    echo "replay_book: $BOOK $D: $(grep -c '^trade,' "$F") trades" >&2
else
    echo "replay_book: backtest failed for $BOOK $D:" >&2; tail -3 "$F.err" >&2
    rm -f "$F.tmp" "$F.err"; exit 4
fi
