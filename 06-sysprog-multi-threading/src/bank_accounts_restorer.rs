use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::thread;
use std::time::Instant;
use rand::prelude::*;
use std::sync::Mutex;

const ACCOUNTS: usize = 50_000_000;
const TRANSACTIONS: usize = 5_000_000_00;
const MAX_TRANSFER: i64 = 10_000_000;
const THREAD_COUNT: usize = 100;

#[derive(Debug)]
struct Transaction {
    from: usize,
    to: usize,
    amount: i64,
}

#[derive(Debug, Copy, Clone)]
struct Account {
    balance: i64,
    checksum: u64,
}

impl Account {
    fn new() -> Self {
        let mut account = Account {
            balance: 0,
            checksum: 0,
        };
        account.update_checksum();
        account
    }

    fn add_balance(&mut self, amount: i64) {
        self.balance += amount;
        self.update_checksum();
    }

    fn calculate_checksum(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.balance.hash(&mut hasher);
        hasher.finish()
    }

    fn update_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }

    fn is_valid(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }
}

struct BankAccountsRestorer {
    accounts: Vec<Mutex<Account>>,
}

impl BankAccountsRestorer {
    fn new() -> Self {
        let mut accounts = Vec::new();
        for _ in 0..ACCOUNTS {
            accounts.push(Mutex::new(Account::new()));
        }
        BankAccountsRestorer {
            accounts: accounts,
        }
    }

    fn get_transaction(&self, num: usize) -> Transaction {
        if num >= TRANSACTIONS {
            panic!("Invalid transaction number: {}", num);
        }

        let mut rng = StdRng::seed_from_u64(num as u64);
        Transaction {
            from: rng.gen_range(0..ACCOUNTS / 2),
            to: rng.gen_range(ACCOUNTS / 2..ACCOUNTS),
            // Objective 4: What happens if instead of above, the following code is set:
            // from: rng.gen_range(0..ACCOUNTS),
            // to: rng.gen_range(0..ACCOUNTS),
            amount: rng.gen_range(-MAX_TRANSFER..MAX_TRANSFER),
        }
    }

    fn run_transactions(&mut self) {
        let threads = THREAD_COUNT;
        let chunk = (TRANSACTIONS + threads - 1) / threads;
        let this = &*self;

        thread::scope(|scope| {
            for t in 0..threads {
                let start = t * chunk;
                let end = (start + chunk).min(TRANSACTIONS);
                scope.spawn(move || {
                    for i in start..end {
                        let tr = this.get_transaction(i);
                        println!("Transaction {}: from {} to {} amount {}", i, tr.from, tr.to, tr.amount);
                        let mut from = this.accounts[tr.from].lock().unwrap();
                        let mut to = this.accounts[tr.to].lock().unwrap();
                        from.add_balance(-tr.amount);
                        to.add_balance(tr.amount);
                    }
                });
            }
        });
    }

    fn check_balances(&self) {
        let mut sum = 0i64;
        let mut abs_sum = 0i64;

        for (i, account) in self.accounts.iter().enumerate() {
            let account = account.lock().unwrap();

            sum += account.balance;
            abs_sum += account.balance.abs();
            if !account.is_valid() {
                panic!(
                    "Invalid checksum for account {}: {} != {}",
                    i,
                    account.calculate_checksum(),
                    account.checksum
                );
            }
        }

        if sum != 0 {
            panic!("Invalid account balance sum: {}", sum);
        }
        if abs_sum != 1026053736714140 {
            panic!("Invalid absolute account total: {}", abs_sum);
        }
    }
}

fn main() {
    let mut bank = BankAccountsRestorer::new();

    let start_time = Instant::now();
    bank.run_transactions();
    let duration = start_time.elapsed();
    println!("Execution time: {:?}", duration);

    bank.check_balances();
}
