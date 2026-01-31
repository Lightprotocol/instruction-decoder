use instruction_decoder_tests::{
    decode_transaction, decode_transaction_snapshot, format_transaction, LiteSVM,
};
use light_instruction_decoder::EnhancedLoggingConfig;
use solana_keypair::{keypair_from_seed, Keypair};
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
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
fn test_decode_transfer() {
    let (mut svm, payer) = setup();
    let recipient = deterministic_keypair(2);
    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), LAMPORTS_PER_SOL);
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let result = svm.send_transaction(versioned_tx.clone());
    let config = EnhancedLoggingConfig::default();
    let snapshot = decode_transaction_snapshot(&versioned_tx, &result, &config);

    assert_eq!(snapshot.status, "Success");
    assert_eq!(snapshot.instructions.len(), 1);
    assert_eq!(snapshot.instructions[0].program_name, "System Program");
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Transfer")
    );

    let log = decode_transaction(&versioned_tx, &result, &config);
    let formatted = format_transaction(&log, &config, 1);
    eprintln!("{formatted}");

    insta::assert_json_snapshot!("transfer", snapshot);
}

#[test]
fn test_decode_create_account() {
    let (mut svm, payer) = setup();
    let new_account = deterministic_keypair(3);
    let owner = deterministic_keypair(4);
    let ix = system_instruction::create_account(
        &payer.pubkey(),
        &new_account.pubkey(),
        LAMPORTS_PER_SOL,
        100,
        &owner.pubkey(),
    );
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &new_account], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let result = svm.send_transaction(versioned_tx.clone());
    let config = EnhancedLoggingConfig::default();
    let snapshot = decode_transaction_snapshot(&versioned_tx, &result, &config);

    assert_eq!(snapshot.status, "Success");
    assert_eq!(snapshot.instructions.len(), 1);
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("CreateAccount")
    );

    insta::assert_json_snapshot!("create_account", snapshot);
}

#[test]
fn test_decode_allocate_and_assign() {
    let (mut svm, payer) = setup();
    let account = deterministic_keypair(5);
    let owner = deterministic_keypair(6);

    let allocate_ix = system_instruction::allocate(&account.pubkey(), 200);
    let assign_ix = system_instruction::assign(&account.pubkey(), &owner.pubkey());
    let msg = Message::new(&[allocate_ix, assign_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer, &account], msg, svm.latest_blockhash());
    let versioned_tx = solana_transaction::versioned::VersionedTransaction::from(tx);

    let result = svm.send_transaction(versioned_tx.clone());
    let config = EnhancedLoggingConfig::default();
    let snapshot = decode_transaction_snapshot(&versioned_tx, &result, &config);

    assert_eq!(snapshot.status, "Success");
    assert_eq!(snapshot.instructions.len(), 2);
    assert_eq!(
        snapshot.instructions[0].instruction_name.as_deref(),
        Some("Allocate")
    );
    assert_eq!(
        snapshot.instructions[1].instruction_name.as_deref(),
        Some("Assign")
    );

    insta::assert_json_snapshot!("allocate_and_assign", snapshot);
}
