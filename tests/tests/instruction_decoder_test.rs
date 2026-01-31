use light_instruction_decoder::{DecoderRegistry, InstructionDecoder};
use sha2::{Digest, Sha256};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

const COUNTER_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Counter111111111111111111111111111111111111");

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}").as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

fn make_accounts(names: &[&str]) -> Vec<AccountMeta> {
    names
        .iter()
        .map(|_| AccountMeta::new(Pubkey::new_unique(), false))
        .collect()
}

#[test]
fn test_decoder_program_id_and_name() {
    let decoder = counter::CounterInstructionDecoder;
    assert_eq!(decoder.program_id(), COUNTER_PROGRAM_ID);
    assert_eq!(decoder.program_name(), "Counter");
}

#[test]
fn test_decoder_can_be_registered() {
    let mut registry = DecoderRegistry::new();
    registry.register(Box::new(counter::CounterInstructionDecoder));
    assert!(registry.has_decoder(&COUNTER_PROGRAM_ID));
}

#[test]
fn test_decoder_decodes_initialize() {
    let decoder = counter::CounterInstructionDecoder;
    let disc = anchor_discriminator("initialize");
    let accounts = make_accounts(&["counter", "authority", "system_program"]);

    let result = decoder.decode(&disc, &accounts);
    assert!(result.is_some());

    let decoded = result.unwrap();
    assert_eq!(decoded.name, "Initialize");
    assert_eq!(
        decoded.account_names,
        vec!["counter", "authority", "system_program"]
    );
}

#[test]
fn test_decoder_decodes_set_with_params() {
    let decoder = counter::CounterInstructionDecoder;
    let disc = anchor_discriminator("set");

    let value: u64 = 42;
    let mut data = disc.to_vec();
    data.extend_from_slice(&value.to_le_bytes());

    let accounts = make_accounts(&["counter", "authority"]);

    let result = decoder.decode(&data, &accounts);
    assert!(result.is_some());

    let decoded = result.unwrap();
    assert_eq!(decoded.name, "Set");
    assert!(decoded.fields.iter().any(|f| f.name == "value" && f.value == "42"));
}

#[test]
fn test_decoder_returns_none_for_unknown() {
    let decoder = counter::CounterInstructionDecoder;
    let bogus = [0xFF; 8];
    let accounts = make_accounts(&["a"]);

    let result = decoder.decode(&bogus, &accounts);
    assert!(result.is_none());
}

#[test]
fn test_discriminators_match_anchor() {
    let decoder = counter::CounterInstructionDecoder;

    let instructions = [
        "initialize",
        "increment",
        "decrement",
        "set",
        "configure",
    ];

    let expected_names = [
        "Initialize",
        "Increment",
        "Decrement",
        "Set",
        "Configure",
    ];

    for (ix_name, expected_name) in instructions.iter().zip(expected_names.iter()) {
        let disc = anchor_discriminator(ix_name);
        let accounts = make_accounts(&["a", "b", "c"]);

        let result = decoder.decode(&disc, &accounts);
        assert!(
            result.is_some(),
            "Decoder should recognize discriminator for '{ix_name}'"
        );
        assert_eq!(
            result.unwrap().name,
            *expected_name,
            "Instruction name mismatch for '{ix_name}'"
        );
    }
}
