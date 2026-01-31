use instruction_decoder_tests::{
    capture_account_states, decode_transaction, format_transaction, strip_ansi_codes,
    TransactionLogger, LiteSVM,
};
use light_instruction_decoder::EnhancedLoggingConfig;
use solana_keypair::{keypair_from_seed, Keypair};
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction;

fn deterministic_keypair(seed_byte: u8) -> Keypair {
    keypair_from_seed(&[seed_byte; 32]).unwrap()
}

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let payer = deterministic_keypair(1);
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    (svm, payer)
}

#[test]
fn test_transaction_logger_transfer() {
    let (mut svm, payer) = setup();
    let recipient = deterministic_keypair(2);

    let config = EnhancedLoggingConfig::debug();
    let logger = TransactionLogger::new(config.clone());

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), LAMPORTS_PER_SOL);
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    // Manually capture states so we can also assert the formatted output
    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = logger.send_transaction(&mut svm, versioned_tx.clone());
    assert!(result.is_ok());
    let post_states = capture_account_states(&svm, &versioned_tx);

    // Re-decode with captured states (logger already logged, but we need the output for snapshot)
    let log = decode_transaction(
        &versioned_tx,
        &result,
        &config,
        Some(&pre_states),
        Some(&post_states),
    );

    // Assert account states are populated
    assert!(log.account_states.is_some());
    let states = log.account_states.as_ref().unwrap();
    let payer_state = states.get(&payer.pubkey()).unwrap();
    assert!(payer_state.lamports_after < payer_state.lamports_before);
    let recipient_state = states.get(&recipient.pubkey()).unwrap();
    assert_eq!(
        recipient_state.lamports_after - recipient_state.lamports_before,
        LAMPORTS_PER_SOL,
    );

    // Snapshot the complete formatted output
    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("logger_transfer_table", stripped);
}

#[test]
fn test_transaction_logger_multiple_transactions() {
    let (mut svm, payer) = setup();
    let recipient = deterministic_keypair(2);

    let config = EnhancedLoggingConfig::debug();
    let logger = TransactionLogger::new(config.clone());

    // Send two transfers
    for i in 0..2 {
        let ix = system_instruction::transfer(
            &payer.pubkey(),
            &recipient.pubkey(),
            LAMPORTS_PER_SOL / (i + 1),
        );
        let msg = Message::new(&[ix], Some(&payer.pubkey()));
        let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
        let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);
        let result = logger.send_transaction(&mut svm, versioned_tx);
        assert!(result.is_ok());
    }
}

#[test]
fn test_account_state_capture_shows_lamport_changes() {
    let (mut svm, payer) = setup();
    let recipient = deterministic_keypair(2);
    let transfer_amount = LAMPORTS_PER_SOL;

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), transfer_amount);
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    assert!(result.is_ok());

    // Verify pre/post state diffs
    let payer_pre = pre_states.get(&payer.pubkey()).unwrap();
    let payer_post = post_states.get(&payer.pubkey()).unwrap();
    assert!(payer_post.0 < payer_pre.0);

    let recipient_pre = pre_states.get(&recipient.pubkey()).unwrap();
    let recipient_post = post_states.get(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_post.0 - recipient_pre.0, transfer_amount);

    let config = EnhancedLoggingConfig::debug();
    let log = decode_transaction(
        &versioned_tx,
        &result,
        &config,
        Some(&pre_states),
        Some(&post_states),
    );

    assert!(log.account_states.is_some());
    let states = log.account_states.as_ref().unwrap();
    let payer_state = states.get(&payer.pubkey()).unwrap();
    assert!(payer_state.lamports_after < payer_state.lamports_before);

    let recipient_state = states.get(&recipient.pubkey()).unwrap();
    assert_eq!(
        recipient_state.lamports_after - recipient_state.lamports_before,
        transfer_amount
    );

    let formatted = format_transaction(&log, &config, 1);
    let stripped = strip_ansi_codes(&formatted);
    insta::assert_snapshot!("account_state_lamport_changes_table", stripped);
}

#[test]
fn test_log_file_is_written() {
    let (mut svm, payer) = setup();
    let recipient = deterministic_keypair(2);

    let config = EnhancedLoggingConfig::default();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), LAMPORTS_PER_SOL);
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let pre_states = capture_account_states(&svm, &versioned_tx);
    let result = svm.send_transaction(versioned_tx.clone());
    let post_states = capture_account_states(&svm, &versioned_tx);

    let log = decode_transaction(
        &versioned_tx,
        &result,
        &config,
        Some(&pre_states),
        Some(&post_states),
    );
    let formatted = format_transaction(&log, &config, 1);
    instruction_decoder_tests::write_to_log_file(&formatted);

    let log_content = std::fs::read_to_string("target/instruction_decoder.log")
        .expect("Log file should exist");
    assert!(!log_content.is_empty(), "Log file should not be empty");
    assert!(
        !log_content.contains("\x1b["),
        "Log file should not contain ANSI escape codes"
    );
}

#[test]
fn test_strip_ansi_codes() {
    let input = "\x1b[32mSuccess\x1b[0m \x1b[1mBold\x1b[0m plain";
    let stripped = strip_ansi_codes(input);
    assert_eq!(stripped, "Success Bold plain");

    let plain = "hello world";
    assert_eq!(strip_ansi_codes(plain), "hello world");
}
