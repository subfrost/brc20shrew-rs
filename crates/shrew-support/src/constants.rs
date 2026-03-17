/// Bitcoin mainnet jubilee height where cursed inscriptions become blessed
pub const JUBILEE_HEIGHT: u32 = 824544;

/// BRC20 activation height (mainnet)
pub const BRC20_ACTIVATION_HEIGHT: u32 = 779832;

/// Runes activation height (mainnet)
pub const RUNES_ACTIVATION_HEIGHT: u32 = 840000;

/// POW20 starting block (mainnet)
pub const POW20_STARTING_BLOCK: u32 = 832486;

/// BRC20 self-mint (5-byte ticker) activation height (mainnet)
pub const BRC20_SELF_MINT_ENABLE_HEIGHT: u32 = 837090;

/// BRC20-prog phase one (6-byte predeploy ticker) activation height (mainnet)
pub const BRC20_PROG_PHASE_ONE_HEIGHT: u32 = 912690;

/// BRC20-prog CANCUN -> PRAGUE hardfork block (mainnet)
pub const BRC20_PROG_PRAGUE_HARDFORK: u32 = 923369;

/// BRC20-prog gas limit per byte of inscription data
pub const BRC20_PROG_GAS_PER_BYTE: u64 = 12_000;

/// BRC20-prog max gas per call
pub const BRC20_PROG_MAX_CALL_GAS: u64 = 1_000_000_000;

/// BRC20-prog max block size in bytes
pub const BRC20_PROG_MAX_BLOCK_SIZE: u64 = 4_000_000;

/// BRC20-prog controller contract address
pub const BRC20_PROG_CONTROLLER_ADDRESS: &str = "0xc54dd4581af2dbf18e4d90840226756e9d2b3cdb";

/// Bitcoin mainnet network
pub const NETWORK: bitcoin::Network = bitcoin::Network::Bitcoin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jubilee_height_value() {
        assert_eq!(JUBILEE_HEIGHT, 824544);
    }

    #[test]
    fn test_runes_activation_height() {
        assert_eq!(RUNES_ACTIVATION_HEIGHT, 840000);
    }

    #[test]
    fn test_brc20_activation_height() {
        assert_eq!(BRC20_ACTIVATION_HEIGHT, 779832);
    }

    #[test]
    fn test_pow20_starting_block() {
        assert_eq!(POW20_STARTING_BLOCK, 832486);
    }

    #[test]
    fn test_brc20_prog_gas_limits() {
        assert!(BRC20_PROG_MAX_CALL_GAS > 0);
        assert!(BRC20_PROG_GAS_PER_BYTE > 0);
    }

    #[test]
    fn test_network_is_mainnet() {
        assert_eq!(NETWORK, bitcoin::Network::Bitcoin);
    }
}
