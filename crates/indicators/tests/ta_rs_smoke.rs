use ta::indicators::ExponentialMovingAverage;
use ta::indicators::RelativeStrengthIndex;
use ta::Next;
use types::test_fixtures::{make_market_state, ranging, trending_up};
use types::Timescale;

#[test]
fn test_ema_on_candles() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(20, 101.0, 1.0));
    let candles = ms.candles.get(&Timescale::FiveMinute).unwrap();
    let mut ema = ExponentialMovingAverage::new(5).unwrap();

    let mut last_ema = 0.0;
    for candle in candles {
        let item: ta::DataItem = candle.into();
        last_ema = ema.next(&item);
    }

    assert!(last_ema > 115.0, "EMA should be near recent prices, got {last_ema}");
    assert!(last_ema < 121.0, "EMA should be below max price, got {last_ema}");
}

#[test]
fn test_rsi_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(20, 105.0, 5.0));
    let candles = ms.candles.get(&Timescale::FiveMinute).unwrap();
    let mut rsi = RelativeStrengthIndex::new(14).unwrap();

    let mut last_rsi = 0.0;
    for candle in candles {
        let item: ta::DataItem = candle.into();
        last_rsi = rsi.next(&item);
    }

    assert!(last_rsi >= 0.0, "RSI should be >= 0, got {last_rsi}");
    assert!(last_rsi <= 100.0, "RSI should be <= 100, got {last_rsi}");
}
