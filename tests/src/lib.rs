//! LiteSVM integration layer for instruction-decoder snapshot testing.
//!
//! Bridges litesvm's transaction callback API with the instruction decoder library,
//! providing functions to decode transactions and produce JSON-serializable snapshots
//! for insta snapshot testing.

use light_instruction_decoder::{
    types::{get_program_name, EnhancedInstructionLog, EnhancedTransactionLog, TransactionStatus},
    EnhancedLoggingConfig, TransactionFormatter,
};
use litesvm::types::{FailedTransactionMetadata, TransactionResult};
use serde::Serialize;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;

// Re-export for convenience in tests
pub use light_instruction_decoder::EnhancedLoggingConfig as Config;
pub use litesvm::LiteSVM;

// ---------------------------------------------------------------------------
// Transaction decoding
// ---------------------------------------------------------------------------

/// Decode a transaction into an `EnhancedTransactionLog`.
///
/// Resolves all top-level and inner (CPI) instructions, maps account indices
/// to `AccountMeta`, and runs the decoder registry against each instruction.
pub fn decode_transaction(
    tx: &VersionedTransaction,
    result: &TransactionResult,
    config: &EnhancedLoggingConfig,
) -> EnhancedTransactionLog {
    let account_keys = tx.message.static_account_keys();
    let signature = tx.signatures.first().copied().unwrap_or_default();

    // Extract metadata from result (both success and failure carry TransactionMetadata)
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

    // Build top-level instructions
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

        // Attach inner instructions for this top-level instruction
        if let Some(inner_ixs) = meta.inner_instructions.get(ix_index) {
            parse_inner_instructions(inner_ixs, account_keys, &tx.message, config, &mut ix_log);
        }

        log.instructions.push(ix_log);
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
) -> TransactionSnapshot {
    let log = decode_transaction(tx, result, config);
    transaction_log_to_snapshot(&log)
}

fn transaction_log_to_snapshot(log: &EnhancedTransactionLog) -> TransactionSnapshot {
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
///
/// Uses `stack_height` to determine nesting depth. Direct CPI calls from the
/// top-level instruction have `stack_height == 2`, nested CPIs have higher values.
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

        // stack_height 1 = top-level, 2 = direct CPI child, 3+ = deeper nesting
        let depth = (inner_ix.stack_height as usize).saturating_sub(1);
        ix_log.depth = depth;
        ix_log.decode(config);

        if depth <= 1 {
            // Direct child of the top-level instruction
            parent.inner_instructions.push(ix_log);
        } else {
            // Nested CPI -- find the parent at depth-1
            let target_depth = depth - 1;
            if let Some(nested_parent) = EnhancedInstructionLog::find_parent_for_instruction(
                &mut parent.inner_instructions,
                target_depth,
            ) {
                nested_parent.inner_instructions.push(ix_log);
            } else {
                // Fallback: attach to top-level inner instructions
                parent.inner_instructions.push(ix_log);
            }
        }
    }
}
