use crate::tables::*;
use crate::proto::{
    GetBalanceRequest, BalanceResponse, GetBrc20EventsRequest, Brc20EventsResponse, Brc20Event,
    get_brc20_events_request,
};
use shrew_support::InscriptionId;
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;

pub fn get_balance(request: &GetBalanceRequest) -> Result<BalanceResponse, String> {
    let mut response = BalanceResponse::default();
    let balance_bytes = BRC20_BALANCES.select(&format!("{}:{}", request.address, request.ticker).as_bytes().to_vec()).get();
    if !balance_bytes.is_empty() {
        response.balance = String::from_utf8(balance_bytes.to_vec()).unwrap_or_default();
    }
    Ok(response)
}

pub fn get_brc20_events(request: &GetBrc20EventsRequest) -> Result<Brc20EventsResponse, String> {
    let mut response = Brc20EventsResponse::default();
    let query = request.query.as_ref().ok_or("Request must specify a query")?;
    let events_bytes = match query {
        get_brc20_events_request::Query::InscriptionId(proto_id) => {
            let inscription_id = InscriptionId {
                txid: Txid::from_slice(&proto_id.txid).map_err(|e| e.to_string())?,
                index: proto_id.index,
            };
            BRC20_EVENTS.select(&inscription_id.to_string().as_bytes().to_vec()).get()
        }
        get_brc20_events_request::Query::BlockHeight(height) => {
            BRC20_EVENTS.select(&height.to_le_bytes().to_vec()).get()
        }
    };
    if !events_bytes.is_empty() {
        if let Ok(events) = serde_json::from_slice::<Vec<Brc20Event>>(&events_bytes) {
            response.events = events;
        }
    }
    Ok(response)
}
