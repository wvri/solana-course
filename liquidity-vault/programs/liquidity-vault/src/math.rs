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
}
