//! BRC20 Controller Contract
//!
//! Deployed at 0xc54dd4581af2dbf18e4d90840226756e9d2b3cdb
//! Provides mint/burn/balanceOf methods for BRC20 tokens within the EVM.

use revm::primitives::Address;

/// The fixed address of the BRC20 controller contract
pub const CONTROLLER_ADDRESS: Address = {
    let bytes: [u8; 20] = [
        0xc5, 0x4d, 0xd4, 0x58, 0x1a, 0xf2, 0xdb, 0xf1, 0x8e, 0x4d,
        0x90, 0x84, 0x02, 0x26, 0x75, 0x6e, 0x9d, 0x2b, 0x3c, 0xdb,
    ];
    Address::new(bytes)
};

/// BRC20 Controller compiled Solidity bytecode (hex-encoded at compile time).
///
/// Source: brc20-programmable-module/src/brc20_controller/contract/output/BRC20_Controller.bin
/// Contains the full controller with methods:
///   - mint(string ticker, address recipient, uint256 amount)
///   - burn(string ticker, address sender, uint256 amount)
///   - balanceOf(string ticker, address account) -> uint256
const CONTROLLER_BYTECODE_HEX: &str = include_str!("BRC20_Controller.hex");

/// Decode the controller bytecode from hex. Called once at startup.
pub fn controller_bytecode() -> Vec<u8> {
    hex::decode(CONTROLLER_BYTECODE_HEX).expect("BRC20_Controller.hex contains invalid hex")
}

/// Function selectors for controller methods
pub mod selectors {
    /// mint(bytes,address,uint256)
    pub const MINT: [u8; 4] = [0x1f, 0xcf, 0xe1, 0x9c];
    /// burn(bytes,address,uint256)
    pub const BURN: [u8; 4] = [0xdc, 0x9a, 0xe1, 0x7d];
    /// balanceOf(bytes,address)
    pub const BALANCE_OF: [u8; 4] = [0xfc, 0x12, 0x4e, 0xbd];
}
