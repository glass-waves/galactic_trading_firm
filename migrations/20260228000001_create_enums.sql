CREATE TYPE timescale AS ENUM ('1min', '5min', '1hour', '1day', '1month');

CREATE TYPE trade_direction AS ENUM ('long', 'short');

CREATE TYPE exit_reason AS ENUM (
    'filter_alignment',
    'trailing_stop',
    'hard_stop',
    'max_hold_timeout',
    'take_profit',
    'session_close',
    'manual_override',
    'config_change'
);

CREATE TYPE agent_type AS ENUM (
    'agent_1min',
    'agent_5min',
    'agent_hourly',
    'agent_daily',
    'agent_monthly',
    'agent_pm',
    'orchestrator'
);

CREATE TYPE memo_type AS ENUM (
    'observation',
    'recommendation'
);

CREATE TYPE cycle_type AS ENUM (
    'full_pm',
    'checkin'
);

CREATE TYPE mutation_status AS ENUM (
    'proposed',
    'backtesting',
    'validated',
    'promoted',
    'rejected',
    'rolled_back',
    'superseded'
);

CREATE TYPE trade_event_type AS ENUM ('entry', 'exit');

CREATE TYPE change_category AS ENUM (
    'knob_tuned',
    'tool_enabled',
    'tool_disabled',
    'tool_instance_added',
    'tool_instance_removed',
    'weight_adjusted',
    'threshold_adjusted',
    'session_rule_changed',
    'scoring_changed',
    'new_tool_type_deployed'
);
