use metashrew_support::index_pointer::{KeyValuePointer};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct BST<T: KeyValuePointer> {
    ptr: T,
}

#[allow(dead_code)]
impl<T: KeyValuePointer> BST<T> {
    pub fn new(ptr: T) -> Self {
        Self { ptr }
    }
    pub fn at(ptr: T) -> Self {
        Self::new(ptr)
    }

    fn get_mask_pointer(&self) -> T {
        self.ptr.keyword("/mask")
    }

    fn get_mask(&self, partial_key: &[u8]) -> [u8; 32] {
        self.ptr.select(&partial_key.to_vec()).keyword("/mask")
            .get()
            .as_ref()
            .to_vec()
            .try_into()
            .unwrap_or([0u8; 32])
    }

    pub fn mark_path(&mut self, key: &[u8]) {
        for i in 0..key.len() {
            let partial_key = &key[..i];
            let mut ptr = self.ptr.select(&partial_key.to_vec()).keyword("/mask");
            let mut mask = self.get_mask(partial_key);
            
            if !is_set_u256(&mask, key[i] as i32) {
                set_bit_u256(&mut mask, key[i] as i32);
                ptr.set(Arc::new(mask.to_vec()));
            }
        }
    }

    pub fn unmark_path(&mut self, key: &[u8]) {
        for i in (0..key.len()).rev() {
            let partial_key = &key[..i];
            let mut ptr = self.ptr.select(&partial_key.to_vec()).keyword("/mask");
            let mut mask = self.get_mask(partial_key);
            
            if is_set_u256(&mask, key[i] as i32) {
                unset_bit_u256(&mut mask, key[i] as i32);
                
                if is_zero_u256(&mask) {
                    ptr.set(Arc::new(Vec::new()));
                    break;
                } else {
                    ptr.set(Arc::new(mask.to_vec()));
                }
            }
        }
    }

    fn find_boundary_from_partial(&self, key_bytes: &[u8], seek_higher: bool) -> Vec<u8> {
        let mut partial_key = key_bytes.to_vec();
        
        while partial_key.len() < 32 { // Using reasonable max size for keys
            let mask = self.get_mask(&partial_key);
            let symbol = binary_search_u256(&mask, seek_higher);
            
            if symbol == -1 {
                break;
            }
            
            partial_key.push(symbol as u8);
        }
        
        partial_key
    }

    pub fn seek_lower(&self, start: &[u8]) -> Option<Vec<u8>> {
        let mut partial_key = start.to_vec();
        
        while !partial_key.is_empty() {
            let this_key = &partial_key[..partial_key.len() - 1];
            let mut mask = self.get_mask(this_key);
            
            if !mask.iter().all(|&x| x == 0) {
                mask_lower_than(&mut mask, partial_key[this_key.len()]);
                let symbol = binary_search_u256(&mask, false);
                
                if symbol != -1 {
                    let mut new_key = this_key.to_vec();
                    new_key.push(symbol as u8);
                    return Some(self.find_boundary_from_partial(&new_key, false));
                }
            }
            
            partial_key = this_key.to_vec();
        }
        
        None
    }

    pub fn seek_greater(&self, start: &[u8]) -> Option<Vec<u8>> {
        let mut partial_key = start.to_vec();
        
        while !partial_key.is_empty() {
            let this_key = &partial_key[..partial_key.len() - 1];
            let mut mask = self.get_mask(this_key);
            
            if !mask.iter().all(|&x| x == 0) {
                mask_greater_than(&mut mask, partial_key[this_key.len()]);
                let symbol = binary_search_u256(&mask, true);
                
                if symbol != -1 {
                    let mut new_key = this_key.to_vec();
                    new_key.push(symbol as u8);
                    return Some(self.find_boundary_from_partial(&new_key, true));
                }
            }
            
            partial_key = this_key.to_vec();
        }
        
        None
    }

    pub fn set(&mut self, key: &[u8], value: Arc<Vec<u8>>) {
        if value.as_ref().is_empty() {
            self.unmark_path(key);
        } else {
            self.mark_path(key);
        }
        self.ptr.select(&key.to_vec()).set(value);
    }
    pub fn set_value(&mut self, key: u64, value: Arc<Vec<u8>>) {
        self.set(&key.to_be_bytes(), value)
    }

    pub fn get(&self, key: &[u8]) -> Option<Arc<Vec<u8>>> {
        let value = self.ptr.select(&key.to_vec()).get();
        if value.as_ref().is_empty() {
            None
        } else {
            Some(value)
        }
    }
}

pub fn mask_lower_than(v: &mut [u8; 32], position: u8) {
    let mut ary = [0u64; 4];
    for i in 0..4 {
        ary[i] = u64::from_be_bytes(v[i*8..(i+1)*8].try_into().unwrap());
    }
    
    let byte_selected = (position / 64) as usize;
    let bit_selected = position % 64;
    
    ary[byte_selected] &= ((1u64 << bit_selected) - 1) << (64 - bit_selected);
    
    for i in byte_selected+1..4 {
        ary[i] = 0;
    }
    
    for i in 0..4 {
        v[i*8..(i+1)*8].copy_from_slice(&ary[i].to_be_bytes());
    }
}

pub fn mask_greater_than(v: &mut [u8; 32], position: u8) {
    let mut ary = [0u64; 4];
    for i in 0..4 {
        ary[i] = u64::from_be_bytes(v[i*8..(i+1)*8].try_into().unwrap());
    }
    
    let byte_selected = (position / 64) as usize;
    let bit_selected = position % 64;
    
    ary[byte_selected] &= !((1u64 << (bit_selected + 1)) - 1) << (63 - bit_selected);
    
    for i in 0..byte_selected {
        ary[i] = 0;
    }
    
    for i in 0..4 {
        v[i*8..(i+1)*8].copy_from_slice(&ary[i].to_be_bytes());
    }
}

pub fn binary_search_u256(v: &[u8; 32], for_highest: bool) -> i32 {
    let mut ary = [0u64; 4];
    for i in 0..4 {
        ary[i] = u64::from_be_bytes(v[i*8..(i+1)*8].try_into().unwrap());
    }
    
    let left = ary[0] | ary[1];
    let right = ary[2] | ary[3];
    
    if (left | right) == 0 {
        return -1;
    }
    
    if (for_highest || right == 0) && left != 0 {
        binary_search_u128(ary[0], ary[1], for_highest)
    } else {
        128 + binary_search_u128(ary[2], ary[3], for_highest)
    }
}

fn binary_search_u128(high: u64, low: u64, for_highest: bool) -> i32 {
    if (for_highest || low == 0) && high != 0 {
        binary_search_u64(high, for_highest)
    } else {
        64 + binary_search_u64(low, for_highest)
    }
}

fn binary_search_u64(word: u64, for_highest: bool) -> i32 {
    let low = (word & 0xFFFFFFFF) as u32;
    let high = ((word >> 32) & 0xFFFFFFFF) as u32;
    
    if (for_highest || low == 0) && high != 0 {
        binary_search_u32(high, for_highest)
    } else {
        32 + binary_search_u32(low, for_highest)
    }
}

fn binary_search_u32(word: u32, for_highest: bool) -> i32 {
    let low = (word & 0xFFFF) as u16;
    let high = ((word >> 16) & 0xFFFF) as u16;
    
    if (for_highest || low == 0) && high != 0 {
        binary_search_u16(high, for_highest)
    } else {
        16 + binary_search_u16(low, for_highest)
    }
}

fn binary_search_u16(word: u16, for_highest: bool) -> i32 {
    let low = (word & 0xFF) as u8;
    let high = ((word >> 8) & 0xFF) as u8;
    
    if (for_highest || low == 0) && high != 0 {
        binary_search_u8(high, for_highest)
    } else {
        8 + binary_search_u8(low, for_highest)
    }
}

fn binary_search_u8(word: u8, for_highest: bool) -> i32 {
    let high = (word >> 4) & 0x0F;
    let low = word & 0x0F;
    
    if (for_highest || low == 0) && high != 0 {
        binary_search_u4(high, for_highest)
    } else {
        4 + binary_search_u4(low, for_highest)
    }
}

fn binary_search_u4(word: u8, for_highest: bool) -> i32 {
    let high = (word >> 2) & 0x03;
    let low = word & 0x03;
    
    if (for_highest || low == 0) && high != 0 {
        binary_search_u2(high, for_highest)
    } else {
        2 + binary_search_u2(low, for_highest)
    }
}

fn binary_search_u2(word: u8, for_highest: bool) -> i32 {
    let high = (word >> 1) & 0x01;
    let low = word & 0x01;
    
    if (for_highest || low == 0) && high != 0 {
        0
    } else {
        1
    }
}

pub fn set_bit_u256(mask: &mut [u8; 32], position: i32) {
    let byte_position = (position / 8) as usize;
    let bit_position = (position % 8) as u8;
    let new_bit = 1u8 << (7 - bit_position);
    mask[byte_position] |= new_bit;
}

pub fn unset_bit_u256(mask: &mut [u8; 32], position: i32) {
    let byte_position = (position / 8) as usize;
    let bit_position = (position % 8) as u8;
    let bit_mask = !(1u8 << (7 - bit_position));
    mask[byte_position] &= bit_mask;
}

pub fn is_set_u256(mask: &[u8; 32], position: i32) -> bool {
    let byte_position = (position / 8) as usize;
    let bit_position = (position % 8) as u8;
    let bit_mask = 1u8 << (7 - bit_position);
    mask[byte_position] & bit_mask != 0
}

pub fn is_zero_u256(mask: &[u8; 32]) -> bool {
    mask.iter().all(|&x| x == 0)
}

