CREATE TABLE trade_indicator_snapshots (
    id                  BIGSERIAL PRIMARY KEY,
    trade_id            BIGINT NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    event_type          trade_event_type NOT NULL,
    timescale           timescale NOT NULL,
    snapshot_at         TIMESTAMPTZ NOT NULL,
    indicators          JSONB NOT NULL,
    UNIQUE (trade_id, event_type, timescale)
);

CREATE INDEX idx_snapshots_trade ON trade_indicator_snapshots(trade_id);
CREATE INDEX idx_snapshots_timescale ON trade_indicator_snapshots(timescale);
