use crate::brc20::{Brc20Indexer, Brc20Operation};

/// Helper to create BRC20 JSON content bytes
fn brc20_json(op: &str, ticker: &str, fields: &[(&str, &str)]) -> Vec<u8> {
    let mut json = format!(r#"{{ "p": "brc-20", "op": "{}", "tick": "{}""#, op, ticker);
    for (key, value) in fields {
        json.push_str(&format!(r#", "{}": "{}""#, key, value));
    }
    json.push_str(" }");
    json.into_bytes()
}

// Note: amounts are now 18-decimal fixed-point u128.
// "21000000" parses to 21000000 * 10^18 = 21_000_000_000_000_000_000_000_000
const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

#[test]
fn test_parse_deploy_operation() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("deploy", "ordi", &[("max", "21000000"), ("lim", "1000")]);
    let result = indexer.parse_operation(&content, 840000);
    assert_eq!(
        result,
        Some(Brc20Operation::Deploy {
            ticker: "ordi".to_string(),
            max_supply: 21_000_000 * SCALE,
            limit_per_mint: 1000 * SCALE,
            decimals: 18,
            self_mint: false,
        })
    );
}

#[test]
fn test_parse_mint_operation() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("mint", "ordi", &[("amt", "500")]);
    let result = indexer.parse_operation(&content, 840000);
    assert_eq!(
        result,
        Some(Brc20Operation::Mint {
            ticker: "ordi".to_string(),
            amount: 500 * SCALE,
        })
    );
}

#[test]
fn test_parse_transfer_operation() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("transfer", "ordi", &[("amt", "250")]);
    let result = indexer.parse_operation(&content, 840000);
    assert_eq!(
        result,
        Some(Brc20Operation::Transfer {
            ticker: "ordi".to_string(),
            amount: 250 * SCALE,
        })
    );
}

#[test]
fn test_parse_invalid_op() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("burn", "ordi", &[("amt", "100")]);
    let result = indexer.parse_operation(&content, 840000);
    assert_eq!(result, None);
}

#[test]
fn test_parse_missing_ticker() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "max": "21000000", "lim": "1000" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert_eq!(result, None);
}

#[test]
fn test_parse_non_json() {
    let indexer = Brc20Indexer::new();
    let content = b"this is not json at all";
    let result = indexer.parse_operation(content, 840000);
    assert_eq!(result, None);
}

#[test]
fn test_parse_deploy_default_decimals() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("deploy", "pepe", &[("max", "420690000000"), ("lim", "1000")]);
    let result = indexer.parse_operation(&content, 840000);
    match result {
        Some(Brc20Operation::Deploy { decimals, .. }) => {
            assert_eq!(decimals, 18, "Default decimals should be 18 when omitted");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_parse_deploy_with_decimals() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("deploy", "sats", &[("max", "2100000000000000"), ("lim", "100000000"), ("dec", "8")]);
    let result = indexer.parse_operation(&content, 840000);
    match result {
        Some(Brc20Operation::Deploy { decimals, .. }) => {
            assert_eq!(decimals, 8, "Explicit decimals should be 8");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_parse_invalid_amount() {
    let indexer = Brc20Indexer::new();
    let content = brc20_json("mint", "ordi", &[("amt", "not_a_number")]);
    let result = indexer.parse_operation(&content, 840000);
    assert_eq!(result, None, "Non-numeric amount should parse as None");
}
