//! 分块还原原始位图。

pub const BLOCK_SIZE: usize = 8192;

fn tagged_uint<const N: usize>(data: &mut &[u8]) -> Option<usize> {
    let bytes = data.first_chunk::<N>()?;
    let mut value = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        if byte & 0x80 == 0 {
            return None;
        }
        value |= usize::from(byte & 127) << (7 * index);
    }
    *data = data.get(N..)?;
    Some(value)
}

pub fn decoded_len(mut data: &[u8]) -> Option<usize> {
    tagged_uint::<3>(&mut data).filter(|&size| size > 12)
}

fn byte(input: &mut &[u8]) -> Option<u8> {
    let (&value, rest) = input.split_first()?;
    *input = rest;
    if value != 0xff {
        return Some(value);
    }
    let (&code, rest) = input.split_first()?;
    *input = rest;
    match code {
        1 => Some(0x0c),
        2 => Some(0x1c),
        3 => Some(0xff),
        _ => None,
    }
}

fn count(input: &mut &[u8], prefix: u8) -> Option<usize> {
    let mut length = usize::from(prefix);
    if prefix == 15 {
        loop {
            let next = byte(input)?;
            length = length.checked_add(usize::from(next))?;
            if next != 255 {
                break;
            }
        }
    }
    Some(length)
}

fn block(mut input: &[u8], output: &mut [u8]) -> Option<()> {
    let mut position = 0usize;
    while !input.is_empty() {
        let token = byte(&mut input)?;
        let literals = count(&mut input, token >> 4)?;
        let end = position.checked_add(literals)?;
        for dst in output.get_mut(position..end)? {
            *dst = byte(&mut input)?;
        }
        position = end;
        if input.is_empty() {
            return (position == output.len()).then_some(());
        }
        let low = byte(&mut input)?;
        let high = byte(&mut input)?;
        let offset = usize::from(u16::from_le_bytes([low, high]));
        if offset == 0 || offset > position {
            return None;
        }
        let length = count(&mut input, token & 15)?.checked_add(4)?;
        let end = position.checked_add(length)?;
        if end > output.len() {
            return None;
        }
        for index in position..end {
            output[index] = output[index - offset];
        }
        position = end;
    }
    None
}

pub fn decode(
    mut data: &[u8],
    scratch: &mut [u8],
    mut emit: impl FnMut(&[u8]) -> Option<()>,
) -> Option<()> {
    let mut size = decoded_len(data)?;
    data = data.get(3..)?;
    let stride = tagged_uint::<2>(&mut data)?;
    if stride == 0 || stride % 4 != 0 {
        return None;
    }
    while size > 0 {
        let length = tagged_uint::<2>(&mut data)?;
        let (&mode, payload) = data.split_first()?;
        let (encoded, next) = payload.split_at_checked(length)?;
        let chunk = scratch.get_mut(..size.min(BLOCK_SIZE))?;
        block(encoded, chunk)?;
        let shift = match mode {
            0 => chunk.len(),
            1 => 4,
            2 => stride,
            _ => return None,
        };
        for index in shift..chunk.len() {
            chunk[index] = chunk[index].wrapping_add(chunk[index - shift]);
        }
        emit(chunk)?;
        size -= chunk.len();
        data = next;
    }
    data.is_empty().then_some(())
}
