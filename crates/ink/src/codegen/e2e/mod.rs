mod callables;
mod xts;
mod builders;
mod utils;

pub use sp_core::H256;
pub use sp_keyring::AccountKeyring;
pub use subxt::{
    self,
    tx::PairSigner,
};
pub use tokio;

use pallet_contracts_primitives::{
    CodeUploadResult,
    ContractExecResult,
    ContractInstantiateResult,
};
use sp_core::sr25519;
use std::{
    cell::RefCell,
    sync::Once,
};
use xts::ContractsApi;

/// Signer that is used throughout the E2E testing.
///
/// The E2E testing can only be used with nodes that support `sr25519`
/// cryptography.
pub type Signer<C> = PairSigner<C, sr25519::Pair>;

// We save the name of the currently executing test here as a mean
// of prefixing log entries to make it easier pinning them to tests.
thread_local! {
    /// This prefix will be used for log output. It is set by each
    /// `#[ink_e2e::test]` with the function name as String.
    /// This way it is possible to distinguish the lines in stdout
    /// and stderr, to still know which line belongs to which test.
    pub static LOG_PREFIX: RefCell<String> = RefCell::new(String::from("no prefix set"));
}

/// Returns the name of the test which is currently executed.
pub fn log_prefix() -> String {
    LOG_PREFIX.with(|log_prefix| log_prefix.borrow().clone())
}

/// Writes `msg` to stdout.
pub fn log_info(msg: &str) {
    log::info!("[{}] {}", log_prefix(), msg);
}

/// Writes `msg` to stderr.
pub fn log_error(msg: &str) {
    log::error!("[{}] {}", log_prefix(), msg);
}
