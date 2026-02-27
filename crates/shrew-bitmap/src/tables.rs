use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    /// Bitmap number -> InscriptionId bytes (first valid inscription wins)
    pub static ref BITMAP_NUMBER_TO_ID: IndexPointer = IndexPointer::from_keyword("/bitmap/number_to_id/");
    /// InscriptionId -> bitmap number
    pub static ref BITMAP_ID_TO_NUMBER: IndexPointer = IndexPointer::from_keyword("/bitmap/id_to_number/");
    /// Height -> list of bitmap entries inscribed at that height
    pub static ref BITMAP_HEIGHT_TO_ENTRIES: IndexPointer = IndexPointer::from_keyword("/bitmap/height_to_entries/");
}
