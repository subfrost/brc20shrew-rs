use crate::balance_sheet::{BalanceSheet, RuneId};
use crate::rune_indexer::RuneEntry;
use crate::tables::*;
use crate::proto::{
    GetRuneRequest, GetRuneResponse, GetRuneBalanceRequest, GetRuneBalanceResponse,
    GetRuneEventsRequest, GetRuneEventsResponse, get_rune_request,
};
use metashrew_support::index_pointer::KeyValuePointer;

pub fn get_rune(request: &GetRuneRequest) -> Result<GetRuneResponse, String> {
    let mut response = GetRuneResponse::default();
    let query = request.query.as_ref().ok_or("Missing query")?;

    let entry_bytes = match query {
        get_rune_request::Query::Id(proto_id) => {
            let rune_id = RuneId::new(proto_id.block, proto_id.tx);
            RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get()
        }
        get_rune_request::Query::Name(name) => {
            let id_bytes = RUNE_NAME_TO_ID.select(&name.to_uppercase().as_bytes().to_vec()).get();
            if id_bytes.is_empty() { return Ok(response); }
            RUNE_ID_TO_ENTRY.select(&id_bytes).get()
        }
    };

    if entry_bytes.is_empty() { return Ok(response); }

    if let Ok(entry) = bincode::deserialize::<RuneEntry>(&entry_bytes) {
        response.entry = Some(crate::proto::RuneEntry {
            id: Some(crate::proto::RuneId { block: entry.id.block, tx: entry.id.tx }),
            name: entry.name,
            spaced_name: entry.spaced_name,
            divisibility: entry.divisibility as u32,
            symbol: entry.symbol.map(|c| c.to_string()),
            spacers: entry.spacers,
            premine: entry.premine as u64,
            terms: entry.terms.map(|t| crate::proto::RuneTerms {
                amount: t.amount.map(|a| a.to_string()),
                cap: t.cap.map(|c| c.to_string()),
                height_start: t.height_start,
                height_end: t.height_end,
                offset_start: t.offset_start,
                offset_end: t.offset_end,
            }),
            turbo: entry.turbo,
            mints: entry.mints as u64,
            supply: entry.supply.to_string(),
            etching_height: entry.etching_height,
            etching_txid: entry.etching_txid.to_vec(),
        });
    }

    Ok(response)
}

pub fn get_rune_balance(request: &GetRuneBalanceRequest) -> Result<GetRuneBalanceResponse, String> {
    let mut response = GetRuneBalanceResponse::default();
    let mut outpoint_bytes = request.txid.clone();
    outpoint_bytes.extend_from_slice(&request.vout.to_le_bytes());
    let data = RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).get();
    if data.is_empty() { return Ok(response); }
    if let Some(sheet) = BalanceSheet::from_bytes(&data) {
        for (rune_id, amount) in &sheet.balances {
            response.balances.push(crate::proto::RuneBalance {
                rune_id: Some(crate::proto::RuneId { block: rune_id.block, tx: rune_id.tx }),
                amount: amount.to_string(),
            });
        }
    }
    Ok(response)
}

pub fn get_rune_events(request: &GetRuneEventsRequest) -> Result<GetRuneEventsResponse, String> {
    let mut response = GetRuneEventsResponse::default();
    let data = HEIGHT_TO_RUNE_EVENTS.select(&request.block_height.to_le_bytes().to_vec()).get();
    if data.is_empty() { return Ok(response); }
    if let Ok(events) = bincode::deserialize::<Vec<crate::rune_indexer::RuneEvent>>(&data) {
        for event in events {
            response.events.push(crate::proto::RuneEvent {
                event_type: event.event_type,
                rune_id: Some(crate::proto::RuneId { block: event.rune_id.block, tx: event.rune_id.tx }),
                amount: event.amount.to_string(),
                txid: event.txid.to_vec(),
                vout: event.vout,
                address: String::new(),
                block_height: event.block_height,
            });
        }
    }
    Ok(response)
}
