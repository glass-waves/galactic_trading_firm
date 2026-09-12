-- breakeven_stop was a no-op until 2026-09-12 (the engine ignored ModifyStop); it now closes
-- positions and needs its own exit_reason value.
ALTER TYPE exit_reason ADD VALUE IF NOT EXISTS 'breakeven_stop';
