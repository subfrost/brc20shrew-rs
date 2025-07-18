# BRC20 Indexing Implementation Proposal

This document outlines a proposal for extending the existing inscription indexer to support the BRC20 fungible token standard.

## 1. Background

The current indexer is designed to process and catalog ordinal inscriptions, tracking their creation, content, and basic properties. To support BRC20, we need to add functionality to parse BRC20 operation data from inscriptions, manage token states (balances, supply), and validate transactions according to the BRC20 protocol rules.

Our research into existing BRC20 indexers, specifically the `OPI` and `metashrew-brc20` projects, has revealed two primary architectural approaches:

*   **Database-Centric (OPI):** This model uses a relational database (PostgreSQL) to store all BRC20 data, including events, tickers, and balances. The indexing logic is implemented in a Python script that reads block data, processes BRC20 operations, and updates the database accordingly.
*   **State Machine (metashrew-brc20):** This approach treats the BRC20 protocol as a state machine, where each transaction can trigger a state transition. The core logic is implemented on a "metashrew-view" server, and clients interact with it via RPC.

Given our existing architecture, which uses a key-value store, a state machine approach is a more natural fit. It will allow us to build on our current infrastructure without introducing a new database dependency.

## 2. Proposed Architecture

We will extend the `InscriptionIndexer` to include a `Brc20Indexer` component. This component will be responsible for processing BRC20-related inscriptions and maintaining the BRC20 state.

### 2.1. Data Structures

We will introduce the following new data structures, which will be defined in a new `src/brc20.rs` file:

*   **`Brc20Operation`:** An enum representing the three BRC20 operations: `Deploy`, `Mint`, and `Transfer`. Each variant will contain the data relevant to that operation (e.g., `Deploy` will have `ticker`, `max_supply`, `limit_per_mint`, and `decimals`).
*   **`Ticker`:** A struct to store information about a BRC20 token, including its name, maximum supply, current supply, and other relevant data.
*   **`Balance`:** A struct to represent a user's balance of a specific token, including total and available balances.

### 2.2. State Management

The BRC20 state will be managed using the following new tables in our key-value store:

*   **`BRC20_TICKERS`:** Maps a ticker symbol to its `Ticker` struct.
*   **`BRC20_BALANCES`:** Maps a combination of a user's address and a ticker symbol to a `Balance` struct.
*   **`BRC20_EVENTS`:** Stores a log of all BRC20 operations, indexed by transaction ID and event type.

### 2.3. Indexing Logic

The `InscriptionIndexer` will be modified as follows:

1.  **Identify BRC20 Inscriptions:** When processing an inscription, the indexer will check if it has a `metaprotocol` field equal to `brc-20`.
2.  **Parse BRC20 Operation:** If it is a BRC20 inscription, the content will be parsed to determine the operation (`deploy`, `mint`, or `transfer`) and its parameters.
3.  **Dispatch to `Brc20Indexer`:** The parsed operation will be passed to the `Brc20Indexer` for processing.

The `Brc20Indexer` will contain the following logic:

*   **`process_deploy`:** Validates and processes a `deploy` operation. It will check if the ticker already exists and, if not, create a new `Ticker` entry in the `BRC20_TICKERS` table.
*   **`process_mint`:** Validates and processes a `mint` operation. It will check if the mint amount exceeds the per-mint limit and the total supply, and then update the user's balance and the token's total supply.
*   **`process_transfer`:** Validates and processes a `transfer` operation. This is a two-step process:
    1.  **Inscribe Transfer:** When a `transfer` inscription is created, the user's available balance is reduced.
    2.  **Execute Transfer:** When the `transfer` inscription is sent to another address, the sender's total balance is reduced, and the receiver's balance is increased.

## 3. Implementation Plan

The implementation will be carried out in the following phases:

1.  **Phase 1: Core Data Structures and Tables:**
    *   Create `src/brc20.rs` with the `Brc20Operation`, `Ticker`, and `Balance` structs.
    *   Define the new BRC20 tables in `src/tables.rs`.

2.  **Phase 2: BRC20 Parsing and Processing:**
    *   Extend `InscriptionIndexer` to identify and parse BRC20 inscriptions.
    *   Implement the `Brc20Indexer` with the `process_deploy`, `process_mint`, and `process_transfer` functions.

3.  **Phase 3: View Functions:**
    *   Create new view functions to query BRC20 data, such as getting a user's balance for a specific token or retrieving information about a ticker.

4.  **Phase 4: Testing:**
    *   Develop a comprehensive test suite to cover all aspects of the BRC20 indexing logic, including valid and invalid operations, edge cases, and state transitions.

## 4. Conclusion

This proposal outlines a clear path for implementing BRC20 indexing in our existing system. By adopting a state machine architecture and building on our current infrastructure, we can deliver a robust and efficient solution.