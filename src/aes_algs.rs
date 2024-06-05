use aes::Aes128;
use aes::cipher::{generic_array::GenericArray, KeyInit, BlockDecrypt, BlockEncrypt};
use crate::xor::fixed_xor;
use crate::utils::hex_encode;
use std::collections::HashMap;

pub fn aes_ecb_decrypt(encrypted_bytes: &[u8], key_bytes: &[u8]) -> Vec<u8> {
    // Construct blocks of 16 byte size for AES-128
    let mut blocks = Vec::new();
    (0..encrypted_bytes.len()).step_by(16).for_each(|x| {
        blocks.push(GenericArray::clone_from_slice(&encrypted_bytes[x..x + 16]));
    });
    
    // Initialize cipher
    let key = GenericArray::clone_from_slice(key_bytes);
    let cipher = Aes128::new(&key);
    
    //Decrypt, flatten, output
    cipher.decrypt_blocks(&mut blocks);
    blocks.iter().flatten().map(|&x| x as u8).collect()
}

pub fn aes_ecb_encrypt(plaintext_bytes: &[u8], key_bytes: &[u8]) -> Vec<u8> {
    // Construct blocks of 16 byte size for AES-128
    let mut blocks = Vec::new();
    (0..((plaintext_bytes.len()/16) as usize)).step_by(16).for_each(|x| {
        blocks.push(GenericArray::clone_from_slice(&plaintext_bytes[x..x + 16]));
    });
    
    // Initialize cipher
    let key = GenericArray::clone_from_slice(key_bytes);
    let cipher = Aes128::new(&key);
    
    //Decrypt, flatten, output
    cipher.encrypt_blocks(&mut blocks);

    let mut out: Vec<u8> = Vec::new();
    for block in blocks{
        for byte in block{
            out.push(byte)
        }
    }

    out
}

pub fn is_aes_ecb_encrypted(l: &[u8]) -> bool {
    //let l = encrypted_bytes;
    let unencrypted = &pkcs_padding(&l, 16);
    let block_ct = l.len() / 16;
    
    //blocks of 16 bytes
    let blocks: Vec<Vec<u8>> = 
        (0..(block_ct-1))
        .into_iter()
        .map(|i| unencrypted[(i*16)..((i+1)*16)].to_vec())
        .collect();

    //compare blocks. if any blocks are equal, it is ECB encrypted
    for i in 0..blocks.len() {
        if (i+1..blocks.len()).into_iter().any(|j| blocks[i] == blocks[j]) {
            println!("The repeating block is: {:?}", hex_encode(&blocks[i])); 
            return true;
        }
    }
    return false;
}

pub fn aes_cbc_decrypt(encrypted_bytes: &[u8], key_bytes: &[u8], iv_bytes: &[u8]) -> Vec<u8> {
    //Initialize cipher as AES-128 with key and iv
    let key = GenericArray::clone_from_slice(key_bytes);
    let cipher = Aes128::new(&key);
    let mut iv = iv_bytes.to_vec();
    
    //Encrypted bytes from Base 64 decoding of ch10 file, build decrypted
    let mut decrypted: Vec<u8> = Vec::new();

    //Step through encrypted bytes as a block of 16 bytes (4x4 matrix)
    (0..encrypted_bytes.len()).step_by(16).for_each(|x| {
        let ciphertext: Vec<u8> = encrypted_bytes[x..x + 16].to_vec();
        let mut block = GenericArray::clone_from_slice(&ciphertext);

        //Decrypt the block with AES cipher, it becomes 'plaintext'
        cipher.decrypt_block(&mut block);
        let mut plaintext: Vec<u8> = Vec::new();
        for byte in block{
            plaintext.push(byte);
        }
        //XOR 'plaintext' and 'iv'. This block is decrypted
        for byte in fixed_xor(&plaintext, &iv){
            decrypted.push(byte);
        }
        
        //iv updated to ciphertext and saved for next decryption block
        iv = ciphertext.to_vec();
    });
    
    //Output as a string from decrypted bytes
    //String::from_utf8(decrypted).unwrap()
    decrypted
}

pub fn aes_cbc_encrypt(plaintext_bytes: &[u8], key_bytes: &[u8], iv_bytes: &[u8]) -> Vec<u8> {
    //Initialize cipher as AES-128 with key and iv
    let unencrypted = &pkcs_padding(&plaintext_bytes, 16);
    let key = GenericArray::clone_from_slice(key_bytes);
    let cipher = Aes128::new(&key);
    let mut iv = iv_bytes.to_vec();
    
    //Encrypted bytes from Base 64 decoding of ch10 file, build decrypted
    let mut blocks: Vec<u8> = Vec::new();

    //Step through encrypted bytes as a block of 16 bytes (4x4 matrix)
    (0..((unencrypted.len()/16) as usize)).step_by(16).for_each(|x| {
        let plaintext: Vec<u8> = fixed_xor(&iv, &unencrypted[x..x + 16].to_vec());
        let mut block = GenericArray::clone_from_slice(&plaintext);
        cipher.encrypt_block(&mut block);
        iv = Vec::new();
        for byte in block{
            blocks.push(byte);
            iv.push(byte);
        }
    });
    
    //Output as a string from decrypted bytes
    blocks.iter().map(|&x| x as u8).collect()
}

pub fn find_aes_encryption_mode(encrypted: &[u8]) -> String{
    match is_aes_ecb_encrypted(encrypted){
        true => "ECB".to_string(),
        false => "CBC".to_string(),
    }
}

fn pkcs_padding(message: &[u8], block_size: usize) -> Vec<u8>{
    let mut bytes = message.to_vec();
    let pad_ct = (block_size - (message.len() % block_size)) as u8;
    for _ in 0..pad_ct {
        bytes.push(pad_ct)
    }
    bytes
}

fn find_blocksize(key: &[u8]) -> usize {
    //feed identical bytes to find size
    let mut input = [].to_vec();
    let initial_size = aes_ecb_encrypt(&input, &key).len();
    loop {
        input.push(b'A');
        let len = aes_ecb_encrypt(&input, &key).len();
        //check if additional block is added
        if len != initial_size {
            return len - initial_size;
        }
    }
    panic!("Couldn't find block size");
}

pub fn aes_decrypt(encrypted: &[u8]) -> Vec<u8>{
    //assign random key to global variable
    let random_key: [u8; 16] = rand::random();

    //find block size
    let block_size = find_blocksize(&random_key);

    //find encryption mode -- I just use ECB because I didn't actually implement this :(
    let encryption_mode = find_aes_encryption_mode(&encrypted);

    //create dictionary of all responses for last byte
    let mut block: [u8; 16] = [b'A'; 16];
    let mut last_bytes = HashMap::new();
    for byte in 0..=255 {
        block[15] = byte;
        last_bytes.insert(aes_ecb_decrypt(&block, &random_key), byte);
    }

    //match to an output where last byte of first_block = next byte of unknown string 
    let mut outs: Vec<u8> = Vec::new();
    for byte in encrypted {
        block[15] = *byte;
        let a = aes_ecb_decrypt(&block, &random_key);
        let b = last_bytes[&a];
        outs.push(b);
    }

    //return
    outs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use base64_light::base64_decode;

    #[test]
    fn aes_ecb_tester() {
        let base64_s = fs::read_to_string("encrypted/ch07.txt")
            .and_then(|res| Ok(res.replace("\n", "")))
            .expect("Error reading file");
        let encrypted: Vec<u8> = base64_decode(&base64_s);

        let key = "YELLOW SUBMARINE".as_bytes();
        let decrypted: Vec<u8>  = aes_ecb_decrypt(&encrypted, key);
        let enc2: Vec<u8>  = aes_ecb_encrypt(&decrypted, key);
        let dec2: Vec<u8>  = aes_ecb_decrypt(&enc2, key);
        
        assert_eq!(encrypted, enc2);
        assert_eq!(decrypted, dec2);

    }

    #[test]
    fn aes_cbc_tester() {
        let base64_s = fs::read_to_string("encrypted/ch10.txt")
            .and_then(|res| Ok(res.replace("\n", "")))
            .expect("Error reading file");
        let encrypted: Vec<u8> = base64_decode(&base64_s);

        let key = "YELLOW SUBMARINE".as_bytes();
        let iv: [u8; 16] = [0; 16];
        let decrypted: Vec<u8>  = aes_cbc_decrypt(&encrypted, key, &iv);
        let enc2: Vec<u8>  = aes_cbc_encrypt(&decrypted, key, &iv);
        let dec2: Vec<u8>  = aes_cbc_decrypt(&enc2, key, &iv);
        
        assert_eq!(encrypted, enc2);
        assert_eq!(decrypted, dec2);

    }
}