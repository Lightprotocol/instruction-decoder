//! Inlined Light Protocol types for borsh deserialization.
//!
//! These types are copied from `light-compressed-account` and `light-token-interface`
//! with exact field order preserved for borsh layout compatibility.
//!
//! Key changes from the original types:
//! - `light_compressed_account::pubkey::Pubkey` is replaced with `[u8; 32]`
//! - Zero-copy derives removed (not needed for decoding)
//! - Only `BorshDeserialize` and `Debug` derived

use borsh::BorshDeserialize;

// ============================================================================
// Core Primitives
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompressedCpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    pub cpi_context_account_index: u8,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

// ============================================================================
// Account Data Types
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct CompressedAccount {
    /// Original type: `light_compressed_account::pubkey::Pubkey` (a `[u8; 32]` wrapper)
    pub owner: [u8; 32],
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

// ============================================================================
// Packed Account Types
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub read_only: bool,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

// ============================================================================
// Address Params
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct NewAddressParamsAssignedPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
    pub assigned_to_account: bool,
    pub assigned_account_index: u8,
}

// ============================================================================
// Read-Only Types
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct PackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

// ============================================================================
// InAccount (for InvokeCpiWithReadOnly)
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InAccount {
    pub discriminator: [u8; 8],
    pub data_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
}

// ============================================================================
// AccountInfo Types (for InvokeCpiWithAccountInfo)
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InAccountInfo {
    pub discriminator: [u8; 8],
    pub data_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub lamports: u64,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct OutAccountInfo {
    pub discriminator: [u8; 8],
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    pub lamports: u64,
    pub data: Vec<u8>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct CompressedAccountInfo {
    pub address: Option<[u8; 32]>,
    pub input: Option<InAccountInfo>,
    pub output: Option<OutAccountInfo>,
}

// ============================================================================
// Top-Level Instruction Data Types
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InstructionDataInvokeCpiWithReadOnly {
    pub mode: u8,
    pub bump: u8,
    /// Original type: `light_compressed_account::pubkey::Pubkey`
    pub invoking_program_id: [u8; 32],
    pub compress_or_decompress_lamports: u64,
    pub is_compress: bool,
    pub with_cpi_context: bool,
    pub with_transaction_hash: bool,
    pub cpi_context: CompressedCpiContext,
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsAssignedPacked>,
    pub input_compressed_accounts: Vec<InAccount>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct InstructionDataInvokeCpiWithAccountInfo {
    pub mode: u8,
    pub bump: u8,
    /// Original type: `light_compressed_account::pubkey::Pubkey`
    pub invoking_program_id: [u8; 32],
    pub compress_or_decompress_lamports: u64,
    pub is_compress: bool,
    pub with_cpi_context: bool,
    pub with_transaction_hash: bool,
    pub cpi_context: CompressedCpiContext,
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsAssignedPacked>,
    pub account_infos: Vec<CompressedAccountInfo>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

// ============================================================================
// Token Interface Types - Transfer2
// ============================================================================

/// CompressedCpiContext for Transfer2 -- different from the 3-field version used in light_system.
/// This one has only 2 fields.
#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Transfer2CpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    Compress,
    Decompress,
    CompressAndClose,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression {
    pub mode: CompressionMode,
    pub amount: u64,
    pub mint: u8,
    pub source_or_recipient: u8,
    pub authority: u8,
    pub pool_account_index: u8,
    pub pool_index: u8,
    pub bump: u8,
    pub decimals: u8,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct MultiInputTokenDataWithContext {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool,
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MultiTokenTransferOutputData {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool,
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
}

// ============================================================================
// Token Extension Types
// ============================================================================

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdditionalMetadata {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<[u8; 32]>,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressedOnlyExtensionInstructionData {
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
    pub is_frozen: bool,
    pub compression_index: u8,
    pub is_ata: bool,
    pub bump: u8,
    pub owner_index: u8,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RentConfig {
    pub base_rent: u16,
    pub compression_cost: u16,
    pub lamports_per_byte_per_epoch: u8,
    pub max_funded_epochs: u8,
    pub max_top_up: u16,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompressionInfo {
    pub config_account_version: u16,
    pub compress_to_pubkey: u8,
    pub account_version: u8,
    pub lamports_per_write: u32,
    pub compression_authority: [u8; 32],
    pub rent_sponsor: [u8; 32],
    pub last_claimed_slot: u64,
    pub rent_exemption_paid: u32,
    pub _reserved: u32,
    pub rent_config: RentConfig,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub enum ExtensionInstructionData {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18,
    TokenMetadata(TokenMetadataInstructionData),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    Placeholder26,
    Placeholder27,
    Placeholder28,
    Placeholder29,
    Placeholder30,
    CompressedOnly(CompressedOnlyExtensionInstructionData),
    Compressible(CompressionInfo),
}

/// Top-level Transfer2 instruction data.
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct CompressedTokenInstructionDataTransfer2 {
    pub with_transaction_hash: bool,
    pub with_lamports_change_account_merkle_tree_index: bool,
    pub lamports_change_account_merkle_tree_index: u8,
    pub lamports_change_account_owner_index: u8,
    pub output_queue: u8,
    pub max_top_up: u16,
    pub cpi_context: Option<Transfer2CpiContext>,
    pub compressions: Option<Vec<Compression>>,
    pub proof: Option<CompressedProof>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub out_token_data: Vec<MultiTokenTransferOutputData>,
    pub in_lamports: Option<Vec<u64>>,
    pub out_lamports: Option<Vec<u64>>,
    pub in_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
    pub out_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
}

// ============================================================================
// Token Interface Types - MintAction
// ============================================================================

/// CPI context for MintAction (different from Transfer2CpiContext and CompressedCpiContext).
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MintActionCpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_queue_index: u8,
    pub token_out_queue_index: u8,
    pub assigned_account_index: u8,
    pub read_only_address_trees: [u8; 4],
    pub address_tree_pubkey: [u8; 32],
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MintMetadata {
    pub version: u8,
    pub mint_decompressed: bool,
    pub mint: [u8; 32],
    pub mint_signer: [u8; 32],
    pub bump: u8,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MintInstructionData {
    pub supply: u64,
    pub decimals: u8,
    pub metadata: MintMetadata,
    pub mint_authority: Option<[u8; 32]>,
    pub freeze_authority: Option<[u8; 32]>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq, Default)]
pub struct CreateMint {
    pub read_only_address_trees: [u8; 4],
    pub read_only_address_tree_root_indices: [u16; 4],
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Recipient {
    pub recipient: [u8; 32],
    pub amount: u64,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MintToCompressedAction {
    pub token_account_version: u8,
    pub recipients: Vec<Recipient>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateAuthority {
    pub new_authority: Option<[u8; 32]>,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub struct MintToAction {
    pub account_index: u8,
    pub amount: u64,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateMetadataFieldAction {
    pub extension_index: u8,
    pub field_type: u8,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateMetadataAuthorityAction {
    pub extension_index: u8,
    pub new_authority: [u8; 32],
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RemoveMetadataKeyAction {
    pub extension_index: u8,
    pub key: Vec<u8>,
    pub idempotent: u8,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub struct DecompressMintAction {
    pub rent_payment: u8,
    pub write_top_up: u32,
}

#[derive(BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub struct CompressAndCloseMintAction {
    pub idempotent: u8,
}

#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub enum Action {
    MintToCompressed(MintToCompressedAction),
    UpdateMintAuthority(UpdateAuthority),
    UpdateFreezeAuthority(UpdateAuthority),
    MintTo(MintToAction),
    UpdateMetadataField(UpdateMetadataFieldAction),
    UpdateMetadataAuthority(UpdateMetadataAuthorityAction),
    RemoveMetadataKey(RemoveMetadataKeyAction),
    DecompressMint(DecompressMintAction),
    CompressAndCloseMint(CompressAndCloseMintAction),
}

/// Top-level MintAction instruction data.
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MintActionCompressedInstructionData {
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub max_top_up: u16,
    pub create_mint: Option<CreateMint>,
    pub actions: Vec<Action>,
    pub proof: Option<CompressedProof>,
    pub cpi_context: Option<MintActionCpiContext>,
    pub mint: Option<MintInstructionData>,
}
