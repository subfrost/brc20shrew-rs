use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    pub static ref SNS_NAME_TO_ID: IndexPointer = IndexPointer::from_keyword("/sns/name_to_id/");
    pub static ref SNS_NAMESPACE_TO_ID: IndexPointer = IndexPointer::from_keyword("/sns/namespace_to_id/");
    pub static ref SNS_ID_TO_NAME: IndexPointer = IndexPointer::from_keyword("/sns/id_to_name/");
    pub static ref SNS_HEIGHT_TO_NAMES: IndexPointer = IndexPointer::from_keyword("/sns/height_to_names/");
}
