use crate::tables::*;
use crate::proto::*;
use metashrew_support::index_pointer::KeyValuePointer;

pub fn get_pow20_balance(request: &GetPow20BalanceRequest) -> Result<Pow20BalanceResponse, String> {
    let mut response = Pow20BalanceResponse::default();
    let key = format!("{}:{}", request.address, request.ticker.to_lowercase());
    let balance_bytes = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
    if !balance_bytes.is_empty() {
        if let Ok(balance) = serde_json::from_slice::<crate::pow20_indexer::Pow20Balance>(&balance_bytes) {
            response.balance = balance.available_balance.to_string();
        }
    }
    Ok(response)
}

pub fn get_pow20_events(request: &GetPow20EventsRequest) -> Result<Pow20EventsResponse, String> {
    let response = Pow20EventsResponse::default();
    let _data = POW20_EVENTS.select(&request.block_height.to_le_bytes().to_vec()).get();
    // Events stored per block - future implementation
    Ok(response)
}
