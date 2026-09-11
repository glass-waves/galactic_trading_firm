#!/usr/bin/env bash
# update the promoted config in postgres using a jq expression
#
# usage: ./scripts/update_config.sh '<jq_expression>' 'reason' [created_by]
# example: ./scripts/update_config.sh '.scoring.exit_threshold = -0.10' 'looser score exit' claude_intraday
#
# reads the current promoted config, applies the jq transformation, inserts it as a
# NEW promoted row (with promoted_at and parent_version_id set) and supersedes the old
# one. the running paper_trader picks the new row up within 60s, per ticker, as each
# ticker goes flat. created_by must be an agent_type enum value: human (default),
# claude_intraday, claude_eod, orchestrator.
#
# rollback = run this with '.' and a reason while the OLD blob is current, or better:
#   ./scripts/update_config.sh "$(psql "$DATABASE_URL" -tA -c "select config_blob from config_versions where id=<old>")" ...
# — the watcher only sees rows with a HIGHER id than the one it is running, so
# re-promoting an old id in place does nothing to a running process.

set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 '<jq_expression>' 'reason' [created_by]"
    exit 1
fi

JQ_EXPR="$1"
REASON="$2"
CREATED_BY="${3:-human}"

# psql via native binary or the postgres container — see scripts/psql.sh
PSQL=("$(dirname "$0")/psql.sh")

# get current promoted config
OLD_ID=$("${PSQL[@]}" -t -A -c "SELECT id FROM config_versions WHERE status = 'promoted' ORDER BY id DESC LIMIT 1;")
if [[ -z "$OLD_ID" ]]; then
    echo "error: no promoted config found"
    exit 1
fi
OLD_CONFIG=$("${PSQL[@]}" -t -A -c "SELECT config_blob FROM config_versions WHERE id = $OLD_ID;")

# apply jq transformation
NEW_CONFIG=$(echo "$OLD_CONFIG" | jq "$JQ_EXPR")
if [[ -z "$NEW_CONFIG" ]] || [[ "$NEW_CONFIG" == "null" ]]; then
    echo "error: jq expression produced null/empty result"
    exit 1
fi

# insert new config as promoted, supersede old. blob is passed as a psql variable
# so quoting inside the JSON cannot break the SQL. (stdin carries the SQL, so the
# variables go on the command line.)
"${PSQL[@]}" -q \
    -v blob="$NEW_CONFIG" \
    -v reason="$REASON" \
    -v created_by="$CREATED_BY" \
    -v old_id="$OLD_ID" <<'SQL'
BEGIN;
UPDATE config_versions SET status = 'superseded' WHERE id = :old_id;
INSERT INTO config_versions (status, promoted_at, config_blob, created_by, parent_version_id, mutation_reason)
VALUES ('promoted', now(), (:'blob')::jsonb, (:'created_by')::agent_type, :old_id, :'reason');
COMMIT;
SQL

NEW_ID=$("${PSQL[@]}" -t -A -c "SELECT id FROM config_versions WHERE status = 'promoted' ORDER BY id DESC LIMIT 1;")
echo "config updated: row $OLD_ID -> row $NEW_ID ($REASON, by $CREATED_BY)"
