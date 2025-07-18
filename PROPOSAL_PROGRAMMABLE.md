# BRC-20 Programmable Module Integration Proposal

## 1. Overview

This document proposes a plan to integrate a BRC-20 programmable module into the existing `brc20shrew-rs` indexer. The goal is to enable smart contract functionality for BRC-20 tokens, inspired by the `brc20-programmable-module` reference implementation, but adapted for the `metashrew` WASM environment.

This will allow for the creation of decentralized applications (dApps) on top of BRC-20, such as decentralized exchanges (DEXs), lending platforms, and more.

## 2. Core Architecture

The proposed architecture revolves around embedding an EVM (Ethereum Virtual Machine) directly into our `InscriptionIndexer`. This EVM will execute smart contracts inscribed on the Bitcoin blockchain.

### Key Components:

1.  **`ProgrammableBrc20Indexer`:** A new indexer that will wrap the existing `Brc20Indexer`. It will be responsible for:
    *   Identifying `brc20-prog` inscriptions (`deploy`, `call`).
    *   Managing the EVM state.
    *   Executing EVM transactions.
    *   Storing EVM state changes.

2.  **Embedded EVM:** We will use a lightweight, WASM-compatible EVM implementation, such as `revm`, which is also used in the reference implementation.

3.  **Native Precompiles:** Instead of the RPC/HTTP-based precompiles in the reference implementation, we will create native Rust functions that have direct access to the `metashrew` database. This will be more efficient and secure.

4.  **New Database Tables:** We will introduce new `metashrew` tables to store the EVM state, including:
    *   `EVM_ACCOUNTS`: Stores account information (nonce, balance, code hash, storage hash).
    *   `EVM_STORAGE`: Stores contract storage (key-value pairs for each contract).
    *   `CONTRACT_ADDRESS_TO_INSCRIPTION_ID`: Maps EVM contract addresses to their corresponding deploy inscription IDs.

## 3. Inscription Handling

The `ProgrammableBrc20Indexer` will process the following new inscription types:

### `deploy` Operation

*   **Inscription Format:**
    ```json
    {
      "p": "brc20-prog",
      "op": "deploy",
      "d": "<bytecode>"
    }
    ```
*   **Processing Steps:**
    1.  Parse the `deploy` inscription.
    2.  Create a new EVM transaction with the provided bytecode.
    3.  Execute the transaction in the EVM.
    4.  If successful, store the new contract's bytecode and address in the `EVM_ACCOUNTS` and `CONTRACT_ADDRESS_TO_INSCRIPTION_ID` tables.

### `call` Operation

*   **Inscription Format:**
    ```json
    {
      "p": "brc20-prog",
      "op": "call",
      "i": "<inscription_id_of_contract>",
      "d": "<calldata>"
    }
    ```
*   **Processing Steps:**
    1.  Parse the `call` inscription.
    2.  Look up the contract address using the `CONTRACT_ADDRESS_TO_INSCRIPTION_ID` table.
    3.  Create a new EVM transaction targeting the contract address with the provided calldata.
    4.  Execute the transaction in the EVM.
    5.  Store any state changes (e.g., updated storage) in the `EVM_STORAGE` table.

## 4. Native Precompiles

The following precompiles will be implemented as native Rust functions:

*   **`BRC20_Balance` (Address: `0x...ff`)**:
    *   **Input:** BRC-20 ticker, pkscript.
    *   **Logic:** Directly queries the existing `BRC20_BALANCES` table to get the balance.
    *   **Output:** `uint256` balance.

*   **`BTC_Transaction` (Address: `0x...fd`)**:
    *   **Input:** Bitcoin `txid`.
    *   **Logic:** This will be a more complex precompile. For now, we will implement a simplified version that can retrieve basic transaction details if they are stored in our index. A full implementation may require extending the indexer to store more detailed transaction data.

*   **Other Precompiles (`BIP322_Verifier`, `BTC_LastSatLoc`, etc.)**: These will be stubbed out initially and can be implemented in the future as needed. The focus will be on the core BRC-20 functionality first.

## 5. `BRC20_Controller` Contract

We will adapt the `BRC20_Controller.sol` contract from the reference implementation. This contract will be deployed at a fixed address during the indexer's initialization. It will handle the logic for `deposit` and `withdraw` operations, interacting with the `BRC20_Balance` precompile to bridge between the on-chain BRC-20 state and the in-EVM token representation.

## 6. Implementation Plan

1.  **Integrate `revm`:** Add `revm` as a dependency and set up the basic EVM execution environment within a new `programmable_brc20.rs` module.
2.  **Implement New Tables:** Define and implement the new `metashrew` tables for EVM state.
3.  **Implement `ProgrammableBrc20Indexer`:** Create the new indexer and integrate it with the main `InscriptionIndexer`.
4.  **Implement `deploy` and `call`:** Add the logic for processing `deploy` and `call` inscriptions.
5.  **Implement Native Precompiles:** Implement the `BRC20_Balance` precompile.
6.  **Deploy `BRC20_Controller`:** Adapt and deploy the controller contract.
7.  **Implement `deposit` and `withdraw`:** Add the logic for handling `deposit` and `withdraw` inscriptions, which will call the `BRC20_Controller` contract.
8.  **Write Comprehensive Tests:** Create a new test suite (`programmable_brc20_tests.rs`) to cover all new functionality, including contract deployment, calls, and precompile interactions.

## 7. Conclusion

This proposal outlines a clear path to extending our BRC-20 indexer with smart contract capabilities. By leveraging the existing `metashrew` architecture and adapting the concepts from the `brc20-programmable-module`, we can create a powerful and efficient platform for BRC-20 dApps.