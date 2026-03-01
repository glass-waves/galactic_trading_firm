use crate::market::Candle;

impl From<&Candle> for ta::DataItem {
    fn from(candle: &Candle) -> Self {
        ta::DataItem::builder()
            .open(candle.open)
            .high(candle.high)
            .low(candle.low)
            .close(candle.close)
            .volume(candle.volume)
            .build()
            .expect("candle OHLCV values must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ta::Close;
    use ta::High;
    use ta::Low;
    use ta::Open;
    use ta::Volume;

    #[test]
    fn test_candle_to_dataitem() {
        let candle = Candle {
            timestamp: Utc::now(),
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            volume: 1_000_000.0,
        };

        let item: ta::DataItem = (&candle).into();

        assert_eq!(item.open(), candle.open);
        assert_eq!(item.high(), candle.high);
        assert_eq!(item.low(), candle.low);
        assert_eq!(item.close(), candle.close);
        assert_eq!(item.volume(), candle.volume);
    }
}
