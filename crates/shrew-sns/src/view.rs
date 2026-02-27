use crate::tables::*;
use crate::proto::*;
use shrew_support::InscriptionId;
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin_hashes::Hash;

pub fn get_sns_name(request: &GetSnsNameRequest) -> Result<SnsNameResponse, String> {
    let mut response = SnsNameResponse::default();
    let name = request.name.to_lowercase();
    let id_bytes = SNS_NAME_TO_ID.select(&name.as_bytes().to_vec()).get();
    if id_bytes.is_empty() { return Ok(response); }
    if let Ok(id) = InscriptionId::from_bytes(&id_bytes) {
        response.name = name;
        response.inscription_id = Some(crate::proto::InscriptionId {
            txid: id.txid.as_byte_array().to_vec(),
            index: id.index,
        });
    }
    Ok(response)
}

pub fn get_sns_namespace(request: &GetSnsNamespaceRequest) -> Result<SnsNamespaceResponse, String> {
    let mut response = SnsNamespaceResponse::default();
    let ns = request.namespace.to_lowercase();
    let id_bytes = SNS_NAMESPACE_TO_ID.select(&ns.as_bytes().to_vec()).get();
    if id_bytes.is_empty() { return Ok(response); }
    if let Ok(id) = InscriptionId::from_bytes(&id_bytes) {
        response.namespace = ns;
        response.inscription_id = Some(crate::proto::InscriptionId {
            txid: id.txid.as_byte_array().to_vec(),
            index: id.index,
        });
    }
    Ok(response)
}

pub fn get_sns_names_by_height(request: &GetSnsNamesByHeightRequest) -> Result<SnsNamesByHeightResponse, String> {
    let mut response = SnsNamesByHeightResponse::default();
    let entries = SNS_HEIGHT_TO_NAMES.select(&request.block_height.to_le_bytes().to_vec()).get_list();
    for entry_bytes in entries {
        if let Ok(id) = InscriptionId::from_bytes(&entry_bytes) {
            let name_bytes = SNS_ID_TO_NAME.select(&entry_bytes).get();
            let name = String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            response.names.push(SnsNameResponse {
                name,
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
