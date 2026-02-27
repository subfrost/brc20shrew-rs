use bitcoin::Block;
use anyhow::Result;

/// Index a block through the ord inscription indexer
pub fn index_ord_block(block: &Block, height: u32) -> Result<()> {
    let mut indexer = shrew_ord::indexer::InscriptionIndexer::new();
    indexer.load_state().map_err(|e| anyhow::anyhow!("{}", e))?;
    indexer.index_block(block, height).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Index a block through the BRC20 indexer
pub fn index_brc20_block(block: &Block, height: u32) {
    let indexer = shrew_brc20::brc20::Brc20Indexer::new();
    indexer.process_block(block, height);
}

/// Index a block through the runes indexer
pub fn index_runes_block(block: &Block, height: u32) {
    let mut indexer = shrew_runes::rune_indexer::RuneIndexer::new();
    indexer.index_block(block, height);
}

/// Index a block through the bitmap indexer
pub fn index_bitmap_block(block: &Block, height: u32) {
    let indexer = shrew_bitmap::bitmap_indexer::BitmapIndexer::new();
    indexer.index_block(block, height);
}

/// Index a block through the SNS indexer
pub fn index_sns_block(block: &Block, height: u32) {
    let indexer = shrew_sns::sns_indexer::SnsIndexer::new();
    indexer.index_block(block, height);
}

/// Index a block through the POW20 indexer
pub fn index_pow20_block(block: &Block, height: u32) {
    let indexer = shrew_pow20::pow20_indexer::Pow20Indexer::new();
    indexer.index_block(block, height);
}

/// Index a block through the programmable BRC20 indexer
pub fn index_prog_block(block: &Block, height: u32) {
    let mut indexer = shrew_brc20_prog::ProgrammableBrc20Indexer::new();
    indexer.index_block(block, height);
}

/// Run all indexers on a block in sequence
pub fn index_all(block: &Block, height: u32) -> Result<()> {
    index_ord_block(block, height)?;
    index_brc20_block(block, height);
    index_runes_block(block, height);
    index_bitmap_block(block, height);
    index_sns_block(block, height);
    index_pow20_block(block, height);
    index_prog_block(block, height);
    Ok(())
}
