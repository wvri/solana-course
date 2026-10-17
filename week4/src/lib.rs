use anchor_lang::prelude::*;

declare_id!("8dwrHSrYF3deCsWiAp6so9FcvdmNBdJ3shbbWL58QQgD");

/// Простая структура аккаунта (учебный пример Rust для Solana).
pub struct SolanaAccount {
    pub lamports: u64,
    pub owner: String,
}

impl SolanaAccount {
    pub fn new(lamports: u64, owner: String) -> Self {
        SolanaAccount { lamports, owner }
    }

    pub fn check_balance(&self) -> std::result::Result<u64, String> {
        if self.lamports == 0 {
            Err(String::from("Ошибка: баланс равен нулю"))
        } else {
            Ok(self.lamports)
        }
    }
}

pub fn process_instruction(instruction: u8, account: &SolanaAccount) {
    match instruction {
        0 => msg!("Инструкция 0: владелец аккаунта {}", account.owner),
        1 => msg!("Инструкция 1: баланс {} lamports", account.lamports),
        _ => msg!("Ошибка: Неизвестная инструкция!"),
    }
}

#[program]
pub mod rust_basics {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>, data: u64) -> Result<()> {
        let account = SolanaAccount::new(data, "7f3D2rcYgBaHCbsNqpC6ropjceyJB7jZNRTNmyTJtj4z".to_string());

        msg!("--- Тестирование Rust для Solana ---");

        match account.check_balance() {
            Ok(balance) => msg!("Баланс успешно проверен: {} lamports", balance),
            Err(err) => msg!("{}", err),
        }

        process_instruction(0, &account);
        process_instruction(1, &account);
        process_instruction(99, &account);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
