//! Native Solana program decoders using macro-derived implementations.
//!
//! This module contains instruction decoders for native Solana programs
//! that use various discriminator sizes:
//! - 1-byte: SPL Token, Token 2022, Compute Budget, Light Token (CToken)
//! - 4-byte: System Program
//! - 8-byte: Anchor programs (Light Registry, Account Compression, Light System)

// Generic Solana program decoders (always available)
pub mod compute_budget;
pub mod spl_token;
pub mod system;
pub mod token_2022;

pub use compute_budget::ComputeBudgetInstructionDecoder;
pub use spl_token::SplTokenInstructionDecoder;
pub use system::SystemInstructionDecoder;
pub use token_2022::Token2022InstructionDecoder;

// Inlined Light Protocol types for borsh deserialization
pub mod light_types;

// Light Protocol program decoders
pub mod account_compression;
pub mod light_system;
pub mod light_token;
pub mod registry;

pub use account_compression::AccountCompressionInstructionDecoder;
pub use light_token::CTokenInstructionDecoder;
pub use light_system::LightSystemInstructionDecoder;
pub use registry::RegistryInstructionDecoder;
