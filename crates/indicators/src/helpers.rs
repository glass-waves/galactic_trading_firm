use ta::Next;
use types::Candle;

/// replay a sequence of candles through a ta-rs indicator that accepts `Next<f64>` (close price).
/// returns the final output value, or None if the candle slice is empty.
pub fn replay_close<I>(indicator: &mut I, candles: &[Candle]) -> Option<f64>
where
    I: Next<f64, Output = f64>,
{
    let mut last = None;
    for candle in candles {
        last = Some(indicator.next(candle.close));
    }
    last
}

/// replay a sequence of candles through a ta-rs indicator that accepts `Next<&DataItem>`.
/// returns the final output value, or None if the candle slice is empty.
pub fn replay_dataitem<I>(indicator: &mut I, candles: &[Candle]) -> Option<f64>
where
    I: for<'a> Next<&'a ta::DataItem, Output = f64>,
{
    let mut last = None;
    for candle in candles {
        let item: ta::DataItem = candle.into();
        last = Some(indicator.next(&item));
    }
    last
}
