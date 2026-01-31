use instruction_decoder_tests::{
    capture_account_states, decode_transaction, decode_transaction_snapshot, format_transaction,
    strip_ansi_codes, LiteSVM,
};
use light_instruction_decoder::EnhancedLoggingConfig;
use solana_keypair::{keypair_from_seed, Keypair};
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

const COUNTER_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Counter111111111111111111111111111111111111");

fn deterministic_keypair(seed_byte: u8) -> Keypair {
    keypair_from_seed(&[seed_byte; 32]).unwrap()
}

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../target/deploy/counter.so");
    let _ = svm.add_program(COUNTER_PROGRAM_ID, program_bytes);
    let payer = deterministic_keypair(10);
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    (svm, payer)
}

/// Build an Anchor instruction: 8-byte discriminator + borsh-serialized args
fn anchor_ix(
    program_id: &Pubkey,
    discriminator: &[u8; 8],
    data: &[u8],
    accounts: Vec<solana_instruction::AccountMeta>,
) -> solana_instruction::Instruction {
    let mut ix_data = discriminator.to_vec();
    ix_data.extend_from_slice(data);
    solana_instruction::Instruction::new_with_bytes(*program_id, &ix_data, accounts)
}

/// Compute Anchor discriminator: sha256("global:<name>")[..8]
fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}").as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

#[test]
fn test_decode_initialize() {
    let (mut svm, payer) = setup();
    let counter = deterministic_keypair(11);

    let init_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("initialize"),
        &[],
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), true),
            solana_instruction::AccountMeta::new(payer.pubkey(), true),
            solana_instruction::AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
    );

    let msg = Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &counter], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    let config = EnhancedLoggingConfig::debug()
        .with_decoders(vec![Box::new(counter::CounterInstructionDecoder)]);
    let snapshot =
        decode_transaction_snapshot(&versioned_tx, &result, &config, Some(&pre_states), Some(&post_states));

    assert_eq!(snapshot.status, "Success");
    assert_eq!(snapshot.instructions.len(), 1);
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Initialize")
    );
    assert_eq!(snapshot.instructions[0].program_name, "Counter");

    insta::assert_json_snapshot!("counter_initialize", snapshot);

    let log = decode_transaction(
        &versioned_tx, &result, &config, Some(&pre_states), Some(&post_states),
    );
    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("counter_initialize_table", stripped);
}

#[test]
fn test_decode_increment_and_set() {
    let (mut svm, payer) = setup();
    let counter = deterministic_keypair(12);

    // Initialize
    let init_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("initialize"),
        &[],
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), true),
            solana_instruction::AccountMeta::new(payer.pubkey(), true),
            solana_instruction::AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
    );
    let msg = Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &counter], msg, svm.latest_blockhash());
    svm.send_transaction(solana_transaction::versioned::VersionedTransaction::from(
        tx,
    ))
    .unwrap();

    // Increment
    let inc_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("increment"),
        &[],
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), false),
            solana_instruction::AccountMeta::new_readonly(payer.pubkey(), true),
        ],
    );
    let msg = Message::new(&[inc_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    let config = EnhancedLoggingConfig::debug()
        .with_decoders(vec![Box::new(counter::CounterInstructionDecoder)]);
    let snapshot =
        decode_transaction_snapshot(&versioned_tx, &result, &config, Some(&pre_states), Some(&post_states));

    assert_eq!(snapshot.status, "Success");
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Increment")
    );

    insta::assert_json_snapshot!("counter_increment", snapshot);

    let log = decode_transaction(
        &versioned_tx, &result, &config, Some(&pre_states), Some(&post_states),
    );
    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("counter_increment_table", stripped);

    // Set value = 42
    let set_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("set"),
        &42u64.to_le_bytes(),
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), false),
            solana_instruction::AccountMeta::new_readonly(payer.pubkey(), true),
        ],
    );
    let msg = Message::new(&[set_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    let config = EnhancedLoggingConfig::debug()
        .with_decoders(vec![Box::new(counter::CounterInstructionDecoder)]);
    let snapshot =
        decode_transaction_snapshot(&versioned_tx, &result, &config, Some(&pre_states), Some(&post_states));

    assert_eq!(snapshot.status, "Success");
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Set")
    );
    let fields = snapshot.instructions[0].decoded_fields.as_ref().unwrap();
    assert!(fields.iter().any(|f| f.name == "value" && f.value == "42"));

    insta::assert_json_snapshot!("counter_set", snapshot);

    let log = decode_transaction(
        &versioned_tx, &result, &config, Some(&pre_states), Some(&post_states),
    );
    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("counter_set_table", stripped);
}

#[test]
fn test_decode_configure() {
    let (mut svm, payer) = setup();
    let counter = deterministic_keypair(13);

    // Initialize first
    let init_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("initialize"),
        &[],
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), true),
            solana_instruction::AccountMeta::new(payer.pubkey(), true),
            solana_instruction::AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
    );
    let msg = Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &counter], msg, svm.latest_blockhash());
    svm.send_transaction(solana_transaction::versioned::VersionedTransaction::from(
        tx,
    ))
    .unwrap();

    // Build configure instruction data
    let mut data = Vec::new();
    data.extend_from_slice(&999u64.to_le_bytes());
    data.extend_from_slice(&7u16.to_le_bytes());
    data.push(1u8);
    let mut label = [0u8; 32];
    label[..11].copy_from_slice(b"hello_world");
    data.extend_from_slice(&label);
    data.extend_from_slice(&12345u64.to_le_bytes());

    let delegate = deterministic_keypair(20).pubkey();
    let fee_receiver = deterministic_keypair(21).pubkey();
    let config_acc = deterministic_keypair(22).pubkey();
    let metadata = deterministic_keypair(23).pubkey();
    let oracle = deterministic_keypair(24).pubkey();
    let backup_authority = deterministic_keypair(25).pubkey();
    let rent = solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111");

    let configure_ix = anchor_ix(
        &COUNTER_PROGRAM_ID,
        &anchor_discriminator("configure"),
        &data,
        vec![
            solana_instruction::AccountMeta::new(counter.pubkey(), false),
            solana_instruction::AccountMeta::new_readonly(payer.pubkey(), true),
            solana_instruction::AccountMeta::new_readonly(delegate, false),
            solana_instruction::AccountMeta::new(fee_receiver, false),
            solana_instruction::AccountMeta::new_readonly(config_acc, false),
            solana_instruction::AccountMeta::new_readonly(metadata, false),
            solana_instruction::AccountMeta::new_readonly(oracle, false),
            solana_instruction::AccountMeta::new_readonly(backup_authority, false),
            solana_instruction::AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
            solana_instruction::AccountMeta::new_readonly(rent, false),
        ],
    );

    let msg = Message::new(&[configure_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    let config = EnhancedLoggingConfig::debug()
        .with_decoders(vec![Box::new(counter::CounterInstructionDecoder)]);
    let snapshot =
        decode_transaction_snapshot(&versioned_tx, &result, &config, Some(&pre_states), Some(&post_states));

    assert_eq!(snapshot.status, "Success");
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Configure")
    );
    assert_eq!(snapshot.instructions[0].accounts.len(), 10);

    let fields = snapshot.instructions[0].decoded_fields.as_ref().unwrap();
    assert!(fields
        .iter()
        .any(|f| f.name == "new_value" && f.value == "999"));
    assert!(fields
        .iter()
        .any(|f| f.name == "multiplier" && f.value == "7"));
    assert!(fields
        .iter()
        .any(|f| f.name == "nonce" && f.value == "12345"));

    insta::assert_json_snapshot!("counter_configure", snapshot);

    let log = decode_transaction(
        &versioned_tx, &result, &config, Some(&pre_states), Some(&post_states),
    );
    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("counter_configure_table", stripped);
}
