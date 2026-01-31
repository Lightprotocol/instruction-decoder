//! LiteSVM integration for instruction decoding, transaction logging, and snapshot testing.
//!
//! Provides:
//! - [`decode_transaction`] -- decode a transaction into an [`EnhancedTransactionLog`]
//! - [`capture_account_states`] -- capture pre/post account state (lamports, data len)
//! - [`TransactionLogger`] -- one-line API that captures state, sends tx, decodes, formats, and logs
//! - Snapshot types for insta JSON testing
//! - File logging to `target/instruction_decoder.log` (ANSI-stripped)

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Once,
    },
};

use litesvm::{types::TransactionResult, LiteSVM};
use serde::Serialize;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;

use crate::{
    config::EnhancedLoggingConfig,
    formatter::TransactionFormatter,
    types::{
        get_program_name, AccountStateSnapshot, EnhancedInstructionLog, EnhancedTransactionLog,
        TransactionStatus,
    },
};

// ---------------------------------------------------------------------------
// Account state capture
// ---------------------------------------------------------------------------

/// Map of pubkey -> (lamports, data_len, owner) captured from LiteSVM at a point in time.
pub type AccountStates = HashMap<Pubkey, (u64, usize, Pubkey)>;

/// Capture the current account state (lamports, data length, owner) for every account
/// referenced by the transaction.
pub fn capture_account_states(svm: &LiteSVM, tx: &VersionedTransaction) -> AccountStates {
    let account_keys = tx.message.static_account_keys();
    let mut states = HashMap::new();
    for key in account_keys {
        if let Some(account) = svm.get_account(key) {
            states.insert(
                *key,
                (account.lamports, account.data.len(), account.owner),
            );
        } else {
            states.insert(*key, (0, 0, Pubkey::default()));
        }
    }
    states
}

// ---------------------------------------------------------------------------
// Transaction decoding
// ---------------------------------------------------------------------------

/// Decode a transaction with optional pre/post account states.
///
/// When `pre_states` and `post_states` are provided, the returned log's
/// `account_states` field is populated so the formatter can render the
/// 8-column account table with Owner, Data Len, Lamports, and Change columns.
pub fn decode_transaction(
    tx: &VersionedTransaction,
    result: &TransactionResult,
    config: &EnhancedLoggingConfig,
    pre_states: Option<&AccountStates>,
    post_states: Option<&AccountStates>,
) -> EnhancedTransactionLog {
    let mut log = decode_transaction_inner(tx, result, config);

    // Populate account_states from pre/post diffs
    if let (Some(pre), Some(post)) = (pre_states, post_states) {
        let mut snapshots = HashMap::new();
        for (pubkey, &(pre_lamports, pre_data_len, owner)) in pre {
            let (post_lamports, post_data_len, _) = post
                .get(pubkey)
                .copied()
                .unwrap_or((0, 0, Pubkey::default()));
            snapshots.insert(
                *pubkey,
                AccountStateSnapshot {
                    lamports_before: pre_lamports,
                    lamports_after: post_lamports,
                    data_len_before: pre_data_len,
                    data_len_after: post_data_len,
                    owner,
                },
            );
        }
        // Also capture accounts that only appear in post (newly created)
        for (pubkey, &(post_lamports, post_data_len, owner)) in post {
            snapshots.entry(*pubkey).or_insert(AccountStateSnapshot {
                lamports_before: 0,
                lamports_after: post_lamports,
                data_len_before: 0,
                data_len_after: post_data_len,
                owner,
            });
        }
        log.account_states = Some(snapshots);
    }

    log
}

/// Format a decoded transaction log into a human-readable string.
pub fn format_transaction(
    log: &EnhancedTransactionLog,
    config: &EnhancedLoggingConfig,
    tx_number: usize,
) -> String {
    let formatter = TransactionFormatter::new(config);
    formatter.format(log, tx_number)
}

/// Core decode logic shared by both public APIs.
fn decode_transaction_inner(
    tx: &VersionedTransaction,
    result: &TransactionResult,
    config: &EnhancedLoggingConfig,
) -> EnhancedTransactionLog {
    use litesvm::types::FailedTransactionMetadata;

    let account_keys = tx.message.static_account_keys();
    let signature = tx.signatures.first().copied().unwrap_or_default();

    let (status, meta) = match result {
        Ok(meta) => (TransactionStatus::Success, meta),
        Err(FailedTransactionMetadata { err, meta }) => {
            (TransactionStatus::Failed(format!("{err:?}")), meta)
        }
    };

    let mut log = EnhancedTransactionLog::new(signature, 0);
    log.status = status;
    log.compute_used = meta.compute_units_consumed;
    log.fee = (tx.signatures.len() as u64) * 5000;
    log.program_logs_pretty = meta.pretty_logs();

    let registry = config.decoder_registry();
    for (ix_index, compiled_ix) in tx.message.instructions().iter().enumerate() {
        let program_id = account_keys
            .get(compiled_ix.program_id_index as usize)
            .copied()
            .unwrap_or_default();
        let program_name = get_program_name(&program_id, registry);

        let mut ix_log = EnhancedInstructionLog::new(ix_index, program_id, program_name);
        ix_log.data = compiled_ix.data.clone();
        ix_log.accounts = resolve_accounts(&compiled_ix.accounts, account_keys, &tx.message);
        ix_log.depth = 0;
        ix_log.decode(config);

        if let Some(inner_ixs) = meta.inner_instructions.get(ix_index) {
            parse_inner_instructions(inner_ixs, account_keys, &tx.message, config, &mut ix_log);
        }

        log.instructions.push(ix_log);
    }

    log
}

// ---------------------------------------------------------------------------
// Snapshot types (JSON-serializable for insta)
// ---------------------------------------------------------------------------

/// JSON-serializable snapshot of an entire transaction.
#[derive(Debug, Serialize)]
pub struct TransactionSnapshot {
    pub signature: String,
    pub status: String,
    pub fee: u64,
    pub compute_used: u64,
    pub instructions: Vec<InstructionSnapshot>,
}

/// JSON-serializable snapshot of a single instruction (including inner/CPI).
#[derive(Debug, Serialize)]
pub struct InstructionSnapshot {
    pub program_id: String,
    pub program_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_name: Option<String>,
    pub accounts: Vec<AccountSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_fields: Option<Vec<FieldSnapshot>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inner_instructions: Vec<InstructionSnapshot>,
}

/// JSON-serializable snapshot of an account reference within an instruction.
#[derive(Debug, Serialize)]
pub struct AccountSnapshot {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// JSON-serializable snapshot of a decoded instruction field.
#[derive(Debug, Serialize)]
pub struct FieldSnapshot {
    pub name: String,
    pub value: String,
}

/// Convert a decoded transaction log into a JSON-serializable snapshot.
pub fn decode_transaction_snapshot(
    tx: &VersionedTransaction,
    result: &TransactionResult,
    config: &EnhancedLoggingConfig,
    pre_states: Option<&AccountStates>,
    post_states: Option<&AccountStates>,
) -> TransactionSnapshot {
    let log = decode_transaction(tx, result, config, pre_states, post_states);
    transaction_log_to_snapshot(&log)
}

/// Convert an [`EnhancedTransactionLog`] into a [`TransactionSnapshot`].
pub fn transaction_log_to_snapshot(log: &EnhancedTransactionLog) -> TransactionSnapshot {
    TransactionSnapshot {
        signature: log.signature.to_string(),
        status: log.status.text(),
        fee: log.fee,
        compute_used: log.compute_used,
        instructions: log
            .instructions
            .iter()
            .map(instruction_to_snapshot)
            .collect(),
    }
}

fn instruction_to_snapshot(ix: &EnhancedInstructionLog) -> InstructionSnapshot {
    let decoded_fields = ix.decoded_instruction.as_ref().map(|decoded| {
        decoded
            .fields
            .iter()
            .map(|f| FieldSnapshot {
                name: f.name.clone(),
                value: f.value.clone(),
            })
            .collect()
    });

    let accounts: Vec<AccountSnapshot> = ix
        .accounts
        .iter()
        .map(|a| AccountSnapshot {
            pubkey: a.pubkey.to_string(),
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();

    InstructionSnapshot {
        program_id: ix.program_id.to_string(),
        program_name: ix.program_name.clone(),
        instruction_name: ix.instruction_name.clone(),
        accounts,
        decoded_fields,
        inner_instructions: ix
            .inner_instructions
            .iter()
            .map(instruction_to_snapshot)
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// File logging
// ---------------------------------------------------------------------------

static LOG_INIT: Once = Once::new();
static LOG_INIT_DONE: AtomicBool = AtomicBool::new(false);
const LOG_PATH: &str = "target/instruction_decoder.log";

/// Strip ANSI escape codes from text.
pub fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until we find the terminating letter [A-Za-z]
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Append ANSI-stripped content to `target/instruction_decoder.log`.
///
/// On first call per process, the file is truncated (session init).
/// Subsequent calls append.
pub fn write_to_log_file(content: &str) {
    LOG_INIT.call_once(|| {
        // Ensure target/ directory exists
        let _ = fs::create_dir_all("target");
        // Truncate on first write of this session
        if let Ok(mut f) = fs::File::create(LOG_PATH) {
            let _ = f.write_all(b"");
        }
        LOG_INIT_DONE.store(true, Ordering::Release);
    });

    // Wait for init (should be instant after Once completes)
    while !LOG_INIT_DONE.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }

    let stripped = strip_ansi_codes(content);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
        let _ = f.write_all(stripped.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Transaction callback
// ---------------------------------------------------------------------------

/// Create a logging callback that decodes and logs each transaction.
///
/// The returned closure can be called after each `svm.send_transaction()`:
/// ```ignore
/// let callback = create_logging_callback(config);
/// let result = svm.send_transaction(tx.clone());
/// callback(&tx, &result, &svm);
/// ```
///
/// The callback writes to the log file and prints to stderr based on config
/// (failed txs always print; all txs print when `config.log_events` is set).
///
/// Note: Since this fires after the transaction, it cannot capture pre-state.
/// For pre/post account state diffs, use [`TransactionLogger`] instead.
pub fn create_logging_callback(
    config: EnhancedLoggingConfig,
) -> impl Fn(&VersionedTransaction, &TransactionResult, &LiteSVM) {
    let counter = AtomicUsize::new(0);
    move |tx: &VersionedTransaction, result: &TransactionResult, _svm: &LiteSVM| {
        let tx_number = counter.fetch_add(1, Ordering::Relaxed) + 1;
        let log = decode_transaction(tx, result, &config, None, None);
        let formatted = format_transaction(&log, &config, tx_number);

        write_to_log_file(&formatted);

        let should_print = config.log_events || result.is_err();
        if should_print {
            eprint!("{}", formatted);
        }
    }
}

// ---------------------------------------------------------------------------
// TransactionLogger
// ---------------------------------------------------------------------------

/// One-line API for capturing account state, sending transactions, decoding,
/// formatting, and logging.
///
/// # Example
/// ```ignore
/// let logger = TransactionLogger::new(EnhancedLoggingConfig::from_env()
///     .with_decoders(vec![Box::new(MyDecoder)]));
/// let result = logger.send_transaction(&mut svm, tx);
/// ```
pub struct TransactionLogger {
    config: EnhancedLoggingConfig,
    counter: AtomicUsize,
}

impl TransactionLogger {
    /// Create a new logger with the given config.
    pub fn new(config: EnhancedLoggingConfig) -> Self {
        Self {
            config,
            counter: AtomicUsize::new(0),
        }
    }

    /// Capture pre-state, send transaction, capture post-state, decode, format, and log.
    ///
    /// Returns the raw `TransactionResult` so callers can unwrap/assert as needed.
    pub fn send_transaction(
        &self,
        svm: &mut LiteSVM,
        tx: VersionedTransaction,
    ) -> TransactionResult {
        let pre_states = capture_account_states(svm, &tx);
        let result = svm.send_transaction(tx.clone());
        let post_states = capture_account_states(svm, &tx);
        let tx_number = self.counter.fetch_add(1, Ordering::Relaxed) + 1;

        self.log_result(&tx, &result, tx_number, &pre_states, &post_states);
        result
    }

    /// Decode, format, and log a transaction result with pre/post states.
    ///
    /// Called automatically by [`send_transaction`], but can also be called
    /// directly when you manage state capture yourself.
    pub fn log_result(
        &self,
        tx: &VersionedTransaction,
        result: &TransactionResult,
        tx_number: usize,
        pre_states: &AccountStates,
        post_states: &AccountStates,
    ) {
        let log = decode_transaction(tx, result, &self.config, Some(pre_states), Some(post_states));
        let formatted = format_transaction(&log, &self.config, tx_number);

        // Always write to log file
        write_to_log_file(&formatted);

        // Console output: failed txs always print; all txs print when log_events is set
        let should_print = self.config.log_events || result.is_err();
        if should_print {
            eprint!("{}", formatted);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve compiled instruction account indices to `AccountMeta`.
fn resolve_accounts(
    account_indices: &[u8],
    account_keys: &[Pubkey],
    message: &solana_message::VersionedMessage,
) -> Vec<AccountMeta> {
    account_indices
        .iter()
        .map(|&idx| {
            let idx = idx as usize;
            let pubkey = account_keys.get(idx).copied().unwrap_or_default();
            let is_signer = message.is_signer(idx);
            let is_writable = message.is_maybe_writable(idx, None);
            if is_writable {
                AccountMeta::new(pubkey, is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, is_signer)
            }
        })
        .collect()
}

/// Parse inner (CPI) instructions and attach them to the parent instruction log.
fn parse_inner_instructions(
    inner_ixs: &[solana_message::inner_instruction::InnerInstruction],
    account_keys: &[Pubkey],
    message: &solana_message::VersionedMessage,
    config: &EnhancedLoggingConfig,
    parent: &mut EnhancedInstructionLog,
) {
    let registry = config.decoder_registry();

    for (inner_idx, inner_ix) in inner_ixs.iter().enumerate() {
        let program_id = account_keys
            .get(inner_ix.instruction.program_id_index as usize)
            .copied()
            .unwrap_or_default();
        let program_name = get_program_name(&program_id, registry);

        let mut ix_log = EnhancedInstructionLog::new(inner_idx, program_id, program_name);
        ix_log.data = inner_ix.instruction.data.clone();
        ix_log.accounts = resolve_accounts(&inner_ix.instruction.accounts, account_keys, message);

        let depth = (inner_ix.stack_height as usize).saturating_sub(1);
        ix_log.depth = depth;
        ix_log.decode(config);

        if depth <= 1 {
            parent.inner_instructions.push(ix_log);
        } else {
            let target_depth = depth - 1;
            if let Some(nested_parent) = EnhancedInstructionLog::find_parent_for_instruction(
                &mut parent.inner_instructions,
                target_depth,
            ) {
                nested_parent.inner_instructions.push(ix_log);
            } else {
                parent.inner_instructions.push(ix_log);
            }
        }
    }
}
