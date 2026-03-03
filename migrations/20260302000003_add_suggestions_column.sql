-- add structured suggestions column to agent_memos
ALTER TABLE agent_memos ADD COLUMN IF NOT EXISTS suggestions JSONB NOT NULL DEFAULT '[]';
