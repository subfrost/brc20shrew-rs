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

/// Minimal controller contract bytecode.
///
/// In the full implementation, this would contain the compiled Solidity
/// bytecode for the BRC20 controller with methods:
///   - mint(string ticker, address recipient, uint256 amount)
///   - burn(string ticker, address sender, uint256 amount)
///   - balanceOf(string ticker, address account) -> uint256
///
/// For now, we use a placeholder that returns success for any call.
pub const CONTROLLER_BYTECODE: &[u8] = &[
    // PUSH1 0x00 PUSH1 0x00 RETURN (minimal valid bytecode)
    0x60, 0x00, 0x60, 0x00, 0xF3,
];

/// Function selectors for controller methods
pub mod selectors {
    /// mint(string,address,uint256)
    pub const MINT: [u8; 4] = [0x40, 0xc1, 0x0f, 0x19];
    /// burn(string,address,uint256)
    pub const BURN: [u8; 4] = [0x44, 0xdf, 0x8e, 0x70];
    /// balanceOf(string,address)
    pub const BALANCE_OF: [u8; 4] = [0x00, 0xfb, 0xd2, 0x80];
}
