use crate::tables::*;
use crate::proto::*;
use shrew_support::InscriptionId;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;

pub fn get_bitmap(request: &GetBitmapRequest) -> Result<BitmapResponse, String> {
    let mut response = BitmapResponse::default();
    let id_bytes = BITMAP_NUMBER_TO_ID.select(&request.bitmap_number.to_le_bytes().to_vec()).get();
    if id_bytes.is_empty() { return Ok(response); }
    if let Ok(id) = InscriptionId::from_bytes(&id_bytes) {
        response.bitmap_number = request.bitmap_number;
        response.inscription_id = Some(crate::proto::InscriptionId {
            txid: id.txid.as_byte_array().to_vec(),
            index: id.index,
        });
    }
    Ok(response)
}

pub fn get_bitmaps_by_height(request: &GetBitmapsByHeightRequest) -> Result<BitmapsByHeightResponse, String> {
    let mut response = BitmapsByHeightResponse::default();
    let entries = BITMAP_HEIGHT_TO_ENTRIES.select(&request.block_height.to_le_bytes().to_vec()).get_list();
    for entry_bytes in entries {
        if let Ok(id) = InscriptionId::from_bytes(&entry_bytes) {
            let number_bytes = BITMAP_ID_TO_NUMBER.select(&entry_bytes).get();
            let number = if number_bytes.len() >= 8 {
                u64::from_le_bytes(number_bytes[..8].try_into().unwrap_or([0; 8]))
            } else { 0 };
            response.bitmaps.push(BitmapResponse {
                bitmap_number: number,
                inscription_id: Some(crate::proto::InscriptionId {
                    txid: id.txid.as_byte_array().to_vec(),
                    index: id.index,
                }),
                block_height: request.block_height,
            });
        }
    }
    Ok(response)
}
