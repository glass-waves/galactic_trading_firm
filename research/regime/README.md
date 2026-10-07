# SPY regime calendars (2026-09-24)

`make_regime_patches.py` builds `regime_<k>.json` (an event_calendar instance flagging regime days,
no look-ahead) and `regime_days.csv` from `data/bars_iex/SPY.csv`. used by the volume round (plan doc
§15, C/C2) and by pipeline candidate `bear-bounce-sma50`; keep while a candidate references them.
