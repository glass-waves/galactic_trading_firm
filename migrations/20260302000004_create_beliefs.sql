-- accumulated investment beliefs for conceptual verbal reinforcement (CVRF)
CREATE TABLE IF NOT EXISTS beliefs (
    id BIGSERIAL PRIMARY KEY,
    belief_text TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    evidence_count INTEGER NOT NULL DEFAULT 1,
    category VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    source_memo_ids BIGINT[] DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_beliefs_status ON beliefs (status);
CREATE INDEX IF NOT EXISTS idx_beliefs_category ON beliefs (category);
