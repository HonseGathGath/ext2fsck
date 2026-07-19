pub fn count_zeroes(bitmap: &[u8], bg_items: u32) -> u32 { 
    let mut free_count = 0;
    let mut counter = 0;
    
    'here: for (_byte_idx, &byte) in bitmap.iter().enumerate() {
        for i in 0..8 { 
            if counter >= bg_items {
                break 'here;
            }
            counter += 1;
            
            let bit = (byte >> i) & 1;
            if bit == 0 {
                free_count += 1;
            }
        }
    }
    free_count
}

