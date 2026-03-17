///! Controller bytecode and selector tests.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::controller;

#[test]
fn test_controller_bytecode_loads() {
    let bytecode = controller::controller_bytecode();
    assert!(!bytecode.is_empty(), "Controller bytecode should not be empty");
    // Real BRC20_Controller.bin is ~8064 bytes (16128 hex chars / 2)
    assert!(bytecode.len() > 1000, "Controller bytecode should be the full compiled contract, not a placeholder");
}

#[test]
fn test_controller_bytecode_starts_with_valid_evm() {
    let bytecode = controller::controller_bytecode();
    // Solidity-compiled contracts typically start with 0x6080 (PUSH1 0x80) or 0x6060 (PUSH1 0x60)
    assert!(bytecode[0] == 0x60, "Contract bytecode should start with PUSH1 (0x60)");
    assert!(bytecode[1] == 0x80 || bytecode[1] == 0x60,
        "Expected standard Solidity preamble (0x6080 or 0x6060), got 0x60{:02x}", bytecode[1]);
}

#[test]
fn test_controller_address_is_correct() {
    let expected: [u8; 20] = [
        0xc5, 0x4d, 0xd4, 0x58, 0x1a, 0xf2, 0xdb, 0xf1, 0x8e, 0x4d,
        0x90, 0x84, 0x02, 0x26, 0x75, 0x6e, 0x9d, 0x2b, 0x3c, 0xdb,
    ];
    assert_eq!(controller::CONTROLLER_ADDRESS.as_slice(), &expected,
        "Controller address should match 0xc54dd4581af2dbf18e4d90840226756e9d2b3cdb");
}

#[test]
fn test_function_selector_mint() {
    // keccak256("mint(bytes,address,uint256)") first 4 bytes = 0x1fcfe19c
    assert_eq!(controller::selectors::MINT, [0x1f, 0xcf, 0xe1, 0x9c],
        "mint selector should match keccak256('mint(bytes,address,uint256)')");
}

#[test]
fn test_function_selector_burn() {
    // keccak256("burn(bytes,address,uint256)") first 4 bytes = 0xdc9ae17d
    assert_eq!(controller::selectors::BURN, [0xdc, 0x9a, 0xe1, 0x7d],
        "burn selector should match keccak256('burn(bytes,address,uint256)')");
}

#[test]
fn test_function_selector_balance_of() {
    // keccak256("balanceOf(bytes,address)") first 4 bytes = 0xfc124ebd
    assert_eq!(controller::selectors::BALANCE_OF, [0xfc, 0x12, 0x4e, 0xbd],
        "balanceOf selector should match keccak256('balanceOf(bytes,address)')");
}
