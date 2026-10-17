use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};

/// Аккаунт: имя и баланс в lamports (1 SOL = 1_000_000_000 lamports).
#[derive(Debug, Clone)]
struct Account {
    name: String,
    lamports: u64,
}

/// Ошибки приложения (аналог пользовательских ошибок в Solana-программах).
#[derive(Debug)]
enum LedgerError {
    AlreadyExists(String),
    NotFound(String),
    InsufficientFunds { needed: u64, available: u64 },
    InvalidAmount(String),
    BadUsage(&'static str),
    UnknownCommand(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LedgerError::AlreadyExists(n) => write!(f, "аккаунт \"{}\" уже существует", n),
            LedgerError::NotFound(n) => write!(f, "аккаунт \"{}\" не найден", n),
            LedgerError::InsufficientFunds { needed, available } => write!(
                f,
                "недостаточно средств: нужно {}, доступно {}",
                needed, available
            ),
            LedgerError::InvalidAmount(s) => write!(f, "неверная сумма: \"{}\"", s),
            LedgerError::BadUsage(u) => write!(f, "использование: {}", u),
            LedgerError::UnknownCommand(c) => write!(f, "неизвестная команда: \"{}\"", c),
        }
    }
}

/// Реестр аккаунтов.
struct Ledger {
    accounts: HashMap<String, Account>,
}

impl Ledger {
    fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    fn create(&mut self, name: &str, lamports: u64) -> Result<(), LedgerError> {
        if self.accounts.contains_key(name) {
            return Err(LedgerError::AlreadyExists(name.to_string()));
        }
        self.accounts.insert(
            name.to_string(),
            Account {
                name: name.to_string(),
                lamports,
            },
        );
        Ok(())
    }

    fn balance(&self, name: &str) -> Result<u64, LedgerError> {
        self.accounts
            .get(name)
            .map(|a| a.lamports)
            .ok_or_else(|| LedgerError::NotFound(name.to_string()))
    }

    fn transfer(&mut self, from: &str, to: &str, amount: u64) -> Result<(), LedgerError> {
        let available = self.balance(from)?;
        self.balance(to)?;
        if available < amount {
            return Err(LedgerError::InsufficientFunds {
                needed: amount,
                available,
            });
        }
        if let Some(a) = self.accounts.get_mut(from) {
            a.lamports -= amount;
        }
        if let Some(a) = self.accounts.get_mut(to) {
            a.lamports += amount;
        }
        Ok(())
    }

    fn list(&self) -> String {
        let mut accounts: Vec<&Account> = self.accounts.values().collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));
        let mut out = String::new();
        let mut total: u64 = 0;
        for a in accounts {
            out.push_str(&format!("  {:<10} {} lamports\n", a.name, a.lamports));
            total += a.lamports;
        }
        out.push_str(&format!("  Всего: {} lamports", total));
        out
    }
}

fn parse_amount(s: &str) -> Result<u64, LedgerError> {
    s.parse::<u64>()
        .map_err(|_| LedgerError::InvalidAmount(s.to_string()))
}

const HELP: &str = "Команды:
  create <имя> <lamports>          создать аккаунт
  balance <имя>                    показать баланс
  transfer <от> <кому> <lamports>  перевод между аккаунтами
  list                             список аккаунтов
  help                             эта справка
  exit                             выход";

/// Разбор и выполнение одной команды (pattern matching по срезу).
fn execute(ledger: &mut Ledger, line: &str) -> Result<Option<String>, LedgerError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    match parts.as_slice() {
        [] => Ok(None),
        ["create", name, amount] => {
            let lamports = parse_amount(amount)?;
            ledger.create(name, lamports)?;
            Ok(Some(format!("Создан аккаунт {} ({} lamports)", name, lamports)))
        }
        ["create", ..] => Err(LedgerError::BadUsage("create <имя> <lamports>")),
        ["balance", name] => {
            let b = ledger.balance(name)?;
            Ok(Some(format!("Баланс {}: {} lamports", name, b)))
        }
        ["balance", ..] => Err(LedgerError::BadUsage("balance <имя>")),
        ["transfer", from, to, amount] => {
            let lamports = parse_amount(amount)?;
            ledger.transfer(from, to, lamports)?;
            Ok(Some(format!(
                "Переведено {} lamports: {} -> {}",
                lamports, from, to
            )))
        }
        ["transfer", ..] => Err(LedgerError::BadUsage("transfer <от> <кому> <lamports>")),
        ["list"] => Ok(Some(ledger.list())),
        ["help"] => Ok(Some(HELP.to_string())),
        [cmd, ..] => Err(LedgerError::UnknownCommand(cmd.to_string())),
    }
}

fn main() {
    println!("Solana Account CLI. Введите help для списка команд.");
    let mut ledger = Ledger::new();
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // конец ввода
            Ok(_) => {}
            Err(e) => {
                eprintln!("ошибка ввода: {}", e);
                break;
            }
        }

        let line = line.trim();
        if line == "exit" {
            println!("Выход.");
            break;
        }

        match execute(&mut ledger, line) {
            Ok(Some(msg)) => println!("{}", msg),
            Ok(None) => {}
            Err(e) => println!("Ошибка: {}", e),
        }
    }
}
