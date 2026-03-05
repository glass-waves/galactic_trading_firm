#!/usr/bin/env bash
# update the promoted config in postgres using a jq expression
#
# usage: ./scripts/update_config.sh '<jq_expression>' 'reason'
# example: ./scripts/update_config.sh '.scoring.entry_threshold = 0.5' 'lower entry threshold'
#
# reads current promoted config, applies jq transformation, inserts as new promoted version

set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 '<jq_expression>' 'reason'"
    exit 1
fi

JQ_EXPR="$1"
REASON="$2"

# load env
set -a; source "$(dirname "$0")/../.env"; set +a

PSQL="psql -h localhost -p 5433 -U postgres -d adaptive_trading"
export PGPASSWORD=postgres

# get current promoted config
OLD_ID=$($PSQL -t -A -c "SELECT id FROM config_versions WHERE status = 'promoted' ORDER BY id DESC LIMIT 1;")
OLD_CONFIG=$($PSQL -t -A -c "SELECT config_blob FROM config_versions WHERE id = $OLD_ID;")

# apply jq transformation
NEW_CONFIG=$(echo "$OLD_CONFIG" | jq "$JQ_EXPR")

if [[ -z "$NEW_CONFIG" ]] || [[ "$NEW_CONFIG" == "null" ]]; then
    echo "error: jq expression produced null/empty result"
    exit 1
fi

# insert new config as promoted, supersede old
$PSQL -q <<SQL
BEGIN;
UPDATE config_versions SET status = 'superseded' WHERE id = $OLD_ID;
INSERT INTO config_versions (status, config_blob, created_by, parent_version_id, mutation_reason)
VALUES ('promoted', '$(echo "$NEW_CONFIG" | sed "s/'/''/g")'::jsonb, 'orchestrator', $OLD_ID, '$REASON');
COMMIT;
SQL

NEW_ID=$($PSQL -t -A -c "SELECT id FROM config_versions WHERE status = 'promoted' ORDER BY id DESC LIMIT 1;")
echo "config updated: v$OLD_ID -> v$NEW_ID ($REASON)"
