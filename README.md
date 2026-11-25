# Solana Devnet: практические работы 1-13

**Автор:** Масимова Мариям, ИС (каз. отделение)
**Сеть:** Solana Devnet
**Среда:** [Solana Playground](https://beta.solpg.io)

| № | Тема | РО | Отчёт | Ссылка в Explorer |
|---|------|----|-------|-------------------|
| 1 | Вход в Web3: первая транзакция в Solana | РО1 | [week1](week1/README.md) | [tx деплоя](https://explorer.solana.com/tx/5cDCCLv9v9cVeauYSpG5YwyJivyZ2iJQXtoke9hiSSqkzKkF7giN7TaLRaUFq1nTJYmQJb3AoyU7MBxEhwoEUpPm?cluster=devnet) |
| 2 | Как на самом деле работает Solana (Transaction Detective) | РО1 | [week2](week2/README.md) | [tx initialize](https://explorer.solana.com/tx/32ZbJVmhoJ4yH1EVLc9AixiUiSQ8ErSjxd83pne8XfyeAEtiQguDNfcDQbTi9JSvrPvZT4UFF1jtp8HfysPQ3oM1?cluster=devnet) |
| 3 | Rust для Solana-разработчика (CLI) | РО2 | [week3](week3/README.md) | без деплоя |
| 4 | Первая Solana Program | РО2 | [week4](week4/README.md) | [программа](https://explorer.solana.com/address/8dwrHSrYF3deCsWiAp6so9FcvdmNBdJ3shbbWL58QQgD?cluster=devnet) |
| 5 | Anchor Framework: Counter | РО2 | [week5](week5/README.md) | [программа](https://explorer.solana.com/address/2v92GpF5iXQi82UfN77LBPkWEQapt1Vc35JQu9s6LXfg?cluster=devnet) |
| 6 | Accounts, PDAs и архитектура программ | РО2 | [week6](week6/README.md) | [программа](https://explorer.solana.com/address/4kdXLeUUvnWZJqUkYAeKGRvF9LkkR1gvmjd2Mug58D8m?cluster=devnet) |
| 8 | РК1 · BUILD SPRINT: каркас Liquidity Vault | РО2 | [week8](week8/README.md) · [РК1](week8/rk1.md) | после деплоя |
| 9 | Vault: депозит и share-токены | РО2 | [week9](week9/README.md) | после деплоя |
| 10 | CPI и вывод из vault | РО3 | [week10](week10/README.md) | после деплоя |
| 11 | AMM и концентрированная ликвидность (Raydium CLMM) | РО3 | [week11](week11/README.md) | после запуска скриптов |

## Структура

```
weekN/
├── README.md        # отчёт
├── src/             # код программы (lib.rs / main.rs)
├── client/          # клиентские скрипты (client.ts)
└── screenshots/     # скриншоты Explorer и Playground
```

## Сквозной проект: Liquidity Vault (недели 8–13)

Код — Anchor-workspace [`liquidity-vault/`](liquidity-vault/README.md). Каждая неделя — отдельный коммит, отчёты лежат в `weekN/README.md`.

