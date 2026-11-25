//! Математика долей vault. Чистые функции без зависимостей от Anchor —
//! их можно тестировать обычным `cargo test` / `rustc --test`.
//!
//! Цена доли = (total_assets + VIRTUAL_ASSETS) / (total_shares + VIRTUAL_SHARES).
//! Виртуальные единицы делают первую долю 1:1 и не дают делить на ноль.
//! Все промежуточные вычисления — в u128, округление всегда в пользу vault.

pub const VIRTUAL_ASSETS: u128 = 1;
pub const VIRTUAL_SHARES: u128 = 1;

/// Сколько share-токенов выпустить за депозит `assets` (округление вниз).
pub fn shares_for_deposit(assets: u64, total_assets: u64, total_shares: u64) -> Option<u64> {
    let numerator = (assets as u128).checked_mul(total_shares as u128 + VIRTUAL_SHARES)?;
    let denominator = total_assets as u128 + VIRTUAL_ASSETS;
    u64::try_from(numerator / denominator).ok()
}

/// Сколько актива вернуть за сжигание `shares` (округление вниз).
pub fn assets_for_shares(shares: u64, total_assets: u64, total_shares: u64) -> Option<u64> {
    let numerator = (shares as u128).checked_mul(total_assets as u128 + VIRTUAL_ASSETS)?;
    let denominator = total_shares as u128 + VIRTUAL_SHARES;
    u64::try_from(numerator / denominator).ok()
}

/// Позиция CLMM активна (зарабатывает комиссии), если tick_lower <= tick < tick_upper.
pub fn tick_in_range(tick_current: i32, tick_lower: i32, tick_upper: i32) -> bool {
    tick_lower <= tick_current && tick_current < tick_upper
}

/// Изменение `old → new` не превышает `max_bps` (1 bps = 0.01%).
pub fn within_bps(old: u64, new: u64, max_bps: u64) -> bool {
    let diff = (old as i128 - new as i128).unsigned_abs();
    diff * 10_000 <= old as u128 * max_bps as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_deposit_is_one_to_one() {
        assert_eq!(shares_for_deposit(1_000_000, 0, 0), Some(1_000_000));
    }

    #[test]
    fn proportional_after_first_deposit() {
        // 100 активов / 100 долей → депозит 50 даёт 50 долей
        assert_eq!(shares_for_deposit(50, 100, 100), Some(50));
    }

    #[test]
    fn share_price_grows_with_profit() {
        // vault заработал: 200 активов на 100 долей → цена доли ≈ 2
        let s = shares_for_deposit(100, 200, 100).unwrap();
        assert!(s < 100 && s >= 49, "got {s}");
    }

    #[test]
    fn rounds_down_in_favor_of_vault() {
        // 1 * 101 / 201 = 0 — пыль не даёт бесплатных долей
        assert_eq!(shares_for_deposit(1, 200, 100), Some(0));
    }

    #[test]
    fn no_overflow_on_large_values() {
        assert!(shares_for_deposit(u64::MAX, u64::MAX, u64::MAX).is_some());
    }

    #[test]
    fn withdraw_all_returns_everything() {
        assert_eq!(assets_for_shares(150, 150, 150), Some(150));
    }

    #[test]
    fn withdraw_with_profit() {
        // 300 активов на 150 долей: 50 долей ≈ 100 активов (вниз)
        assert_eq!(assets_for_shares(50, 300, 150), Some(99));
    }

    #[test]
    fn deposit_then_withdraw_never_profits() {
        // круговая операция не должна давать прибыль (защита от округления)
        for (a, s) in [(1_000u64, 1_000u64), (1_337, 999), (10_000_007, 3)] {
            for amount in [1u64, 7, 1_000, 123_456] {
                let sh = shares_for_deposit(amount, a, s).unwrap();
                let back = assets_for_shares(sh, a + amount, s + sh).unwrap();
                assert!(back <= amount, "a={a} s={s} amount={amount} back={back}");
            }
        }
    }

    #[test]
    fn range_bounds() {
        assert!(tick_in_range(0, -60, 60));
        assert!(tick_in_range(-60, -60, 60)); // нижняя граница включена
        assert!(!tick_in_range(60, -60, 60)); // верхняя — нет
        assert!(!tick_in_range(-61, -60, 60));
    }

    #[test]
    fn report_bounds() {
        assert!(within_bps(1_000, 1_200, 2_000)); // +20% ок
        assert!(within_bps(1_000, 800, 2_000)); // −20% ок
        assert!(!within_bps(1_000, 1_201, 2_000));
        assert!(!within_bps(1_000, 0, 2_000));
        assert!(within_bps(0, 0, 2_000));
    }
}
